mod dicom_client;

use anyhow::Result;
use chrono::{DateTime, Utc};
use clap::Parser;
use console::{style, Emoji};
use dicom::object::open_file;
use dicom_core::header::Tag;
use dicom_client::{DicomClient, FileTransferResult};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};
use uuid::Uuid;
use walkdir::WalkDir;

static SPARKLE: Emoji<'_, '_> = Emoji("✨ ", "");
static ROCKET: Emoji<'_, '_> = Emoji("🚀 ", "");
static CLIPBOARD: Emoji<'_, '_> = Emoji("📋 ", "");
static STOPWATCH: Emoji<'_, '_> = Emoji("⏱️ ", "");

#[derive(Parser, Clone)]
#[command(name = "dicom-sender")]
#[command(about = "A high-performance DICOM C-STORE sender")]
#[command(version = "1.0")]
struct Args {
    /// Input path (file or directory)
    #[arg(short, long)]
    input: PathBuf,

    /// Recursive directory scanning
    #[arg(short, long)]
    recursive: bool,

    /// Called AE Title (default: RUST_SCU)
    #[arg(short = 'c', long, default_value = "RUST_SCU")]
    calling_ae: String,

    /// Called AE Title (destination)
    #[arg(short = 'a', long)]
    ae_title: String,

    /// Destination IP address
    #[arg(short = 'H', long)]
    host: String,

    /// Destination port
    #[arg(short, long)]
    port: u16,

    /// Number of concurrent threads/associations
    #[arg(short, long, default_value = "1")]
    threads: usize,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DicomFile {
    path: PathBuf,
    study_instance_uid: String,
    series_instance_uid: String,
    sop_instance_uid: String,
    sop_class_uid: String,
    file_size: u64,
    modality: Option<String>,
    patient_id: Option<String>,
    study_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TransferResult {
    file_path: String,
    study_instance_uid: String,
    sop_instance_uid: String,
    success: bool,
    error_message: Option<String>,
    transfer_time_ms: u64,
    file_size: u64,
    timestamp: DateTime<Utc>,
    thread_id: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct SessionSummary {
    session_id: String,
    start_time: DateTime<Utc>,
    end_time: Option<DateTime<Utc>>,
    total_files: usize,
    successful_transfers: usize,
    failed_transfers: usize,
    total_bytes: u64,
    total_time_ms: u64,
    average_transfer_time_ms: f64,
    throughput_mbps: f64,
    threads_used: usize,
    destination: String,
    calling_ae: String,
    called_ae: String,
    studies_processed: Vec<String>,
}

#[derive(Debug)]
struct TransferStats {
    total_files: AtomicU64,
    successful: AtomicU64,
    failed: AtomicU64,
    total_bytes: AtomicU64,
    start_time: Instant,
}

impl TransferStats {
    fn new() -> Self {
        Self {
            total_files: AtomicU64::new(0),
            successful: AtomicU64::new(0),
            failed: AtomicU64::new(0),
            total_bytes: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }

    fn increment_success(&self, bytes: u64) {
        self.successful.fetch_add(1, Ordering::Relaxed);
        self.total_bytes.fetch_add(bytes, Ordering::Relaxed);
    }

    fn increment_failure(&self) {
        self.failed.fetch_add(1, Ordering::Relaxed);
    }

    fn get_throughput_mbps(&self) -> f64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        let bytes = self.total_bytes.load(Ordering::Relaxed) as f64;
        if elapsed > 0.0 {
            (bytes / (1024.0 * 1024.0)) / elapsed
        } else {
            0.0
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    let session_id = Uuid::new_v4().to_string();
    let log_file = format!("dicom_sender_{}.log", session_id);
    let summary_file = format!("dicom_sender_summary_{}.json", session_id);

    tracing_subscriber::fmt()
        .with_writer(
            OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&log_file)?,
        )
        .init();

    println!("{} DICOM Sender v1.0", ROCKET);
    println!("Session ID: {}", style(&session_id).cyan());
    println!("Log file: {}", style(&log_file).yellow());
    println!();

    let start_time = Utc::now();
    let stats = Arc::new(TransferStats::new());

    // Step 1: Index all DICOM files
    println!("{} Indexing DICOM files...", CLIPBOARD);
    let dicom_files = index_dicom_files(&args.input, args.recursive).await?;
    
    if dicom_files.is_empty() {
        println!("❌ No DICOM files found!");
        return Ok(());
    }

    println!("✅ Found {} DICOM files", style(dicom_files.len()).green());

    // Step 2: Group by Study Instance UID
    let mut studies: HashMap<String, Vec<DicomFile>> = HashMap::new();
    for file in &dicom_files {
        studies
            .entry(file.study_instance_uid.clone())
            .or_insert_with(Vec::new)
            .push(file.clone());
    }

    println!("📊 Grouped into {} studies", style(studies.len()).green());
    for (study_uid, files) in &studies {
        println!("  Study: {} ({} files)", 
                 style(&study_uid[..20]).dim(), 
                 style(files.len()).cyan());
    }

    stats.total_files.store(dicom_files.len() as u64, Ordering::Relaxed);

    // Step 3: Setup progress tracking
    let multi_progress = MultiProgress::new();
    let main_progress = multi_progress.add(ProgressBar::new(dicom_files.len() as u64));
    main_progress.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({eta})")?
            .progress_chars("#>-"),
    );

    let results = Arc::new(Mutex::new(Vec::new()));

    // Step 4: Send files using multiple threads
    println!("{} Starting transfer with {} threads...", ROCKET, args.threads);
    
    let study_chunks: Vec<_> = studies.into_iter().collect();
    let chunk_size = (study_chunks.len() + args.threads - 1) / args.threads;
    
    let mut handles: Vec<JoinHandle<Result<()>>> = Vec::new();

    for (thread_id, chunk) in study_chunks.chunks(chunk_size).enumerate() {
        let chunk = chunk.to_vec();
        let args = args.clone();
        let results = results.clone();
        let stats = stats.clone();
        let progress = main_progress.clone();
        let session_id = session_id.clone();

        let handle = tokio::spawn(async move {
            send_studies_worker(
                thread_id,
                chunk,
                &args,
                results,
                stats,
                progress,
                &session_id,
            ).await
        });

        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        if let Err(e) = handle.await? {
            error!("Thread failed: {}", e);
        }
    }

    main_progress.finish_with_message("Transfer completed!");

    let end_time = Utc::now();
    let duration = end_time.signed_duration_since(start_time);

    // Step 5: Generate summary
    let results = results.lock().unwrap();
    let successful = stats.successful.load(Ordering::Relaxed);
    let failed = stats.failed.load(Ordering::Relaxed);
    let total_bytes = stats.total_bytes.load(Ordering::Relaxed);

    let summary = SessionSummary {
        session_id: session_id.clone(),
        start_time,
        end_time: Some(end_time),
        total_files: dicom_files.len(),
        successful_transfers: successful as usize,
        failed_transfers: failed as usize,
        total_bytes,
        total_time_ms: duration.num_milliseconds() as u64,
        average_transfer_time_ms: if successful > 0 {
            results.iter()
                .filter(|r| r.success)
                .map(|r| r.transfer_time_ms)
                .sum::<u64>() as f64 / successful as f64
        } else { 0.0 },
        throughput_mbps: stats.get_throughput_mbps(),
        threads_used: args.threads,
        destination: format!("{}:{}@{}", args.ae_title, args.port, args.host),
        calling_ae: args.calling_ae,
        called_ae: args.ae_title,
        studies_processed: dicom_files.iter()
            .map(|f| f.study_instance_uid.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect(),
    };

    // Write summary to file
    let summary_json = serde_json::to_string_pretty(&summary)?;
    std::fs::write(&summary_file, summary_json)?;

    // Print final statistics
    println!();
    println!("{} Transfer Summary", STOPWATCH);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Total files:     {}", style(summary.total_files).cyan());
    println!("Successful:      {}", style(successful).green());
    println!("Failed:          {}", style(failed).red());
    println!("Total size:      {:.2} MB", total_bytes as f64 / (1024.0 * 1024.0));
    println!("Total time:      {:.2} seconds", duration.num_milliseconds() as f64 / 1000.0);
    println!("Avg transfer:    {:.2} ms", summary.average_transfer_time_ms);
    println!("Throughput:      {:.2} MB/s", summary.throughput_mbps);
    println!("Threads used:    {}", summary.threads_used);
    println!("Studies:         {}", summary.studies_processed.len());
    println!();
    println!("📄 Detailed log: {}", style(&log_file).yellow());
    println!("📊 Summary JSON: {}", style(&summary_file).yellow());

    Ok(())
}

async fn index_dicom_files(input: &Path, recursive: bool) -> Result<Vec<DicomFile>> {
    let mut files = Vec::new();
    
    if input.is_file() {
        if let Some(dicom_file) = process_dicom_file(input).await? {
            files.push(dicom_file);
        }
    } else if input.is_dir() {
        if recursive {
            for entry in WalkDir::new(input) {
                let entry = entry?;
                if entry.file_type().is_file() {
                    let path = entry.path();
                    if let Some(ext) = path.extension() {
                        if ext == "dcm" || ext == "DCM" {
                            if let Some(dicom_file) = process_dicom_file(path).await? {
                                files.push(dicom_file);
                            }
                        }
                    }
                }
            }
        } else {
            for entry in std::fs::read_dir(input)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if ext == "dcm" || ext == "DCM" {
                            if let Some(dicom_file) = process_dicom_file(&path).await? {
                                files.push(dicom_file);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(files)
}

async fn process_dicom_file(path: &Path) -> Result<Option<DicomFile>> {
    match open_file(path) {
        Ok(obj) => {
            let metadata = std::fs::metadata(path)?;
            
            let study_instance_uid = obj
                .element(Tag(0x0020, 0x000D))?
                .to_str()?
                .to_string();

            let series_instance_uid = obj
                .element(Tag(0x0020, 0x000E))?
                .to_str()?
                .to_string();

            let sop_instance_uid = obj
                .element(Tag(0x0008, 0x0018))?
                .to_str()?
                .to_string();

            let sop_class_uid = obj
                .element(Tag(0x0008, 0x0016))?
                .to_str()?
                .to_string();

            let modality = obj
                .element(Tag(0x0008, 0x0060))
                .ok()
                .and_then(|e| e.to_str().ok())
                .map(|s| s.to_string());

            let patient_id = obj
                .element(Tag(0x0010, 0x0020))
                .ok()
                .and_then(|e| e.to_str().ok())
                .map(|s| s.to_string());

            let study_date = obj
                .element(Tag(0x0008, 0x0020))
                .ok()
                .and_then(|e| e.to_str().ok())
                .map(|s| s.to_string());

            Ok(Some(DicomFile {
                path: path.to_path_buf(),
                study_instance_uid,
                series_instance_uid,
                sop_instance_uid,
                sop_class_uid,
                file_size: metadata.len(),
                modality,
                patient_id,
                study_date,
            }))
        }
        Err(e) => {
            warn!("Failed to parse DICOM file {}: {}", path.display(), e);
            Ok(None)
        }
    }
}

async fn send_studies_worker(
    thread_id: usize,
    studies: Vec<(String, Vec<DicomFile>)>,
    args: &Args,
    results: Arc<Mutex<Vec<TransferResult>>>,
    stats: Arc<TransferStats>,
    progress: ProgressBar,
    session_id: &str,
) -> Result<()> {
    info!("Thread {} starting with {} studies", thread_id, studies.len());

    for (study_uid, files) in studies {
        info!("Thread {} processing study: {}", thread_id, study_uid);
        
        // Open association for this study
        match send_study_files(thread_id, &study_uid, files, args, &results, &stats, &progress).await {
            Ok(_) => {
                info!("Thread {} completed study: {}", thread_id, study_uid);
            }
            Err(e) => {
                error!("Thread {} failed study {}: {}", thread_id, study_uid, e);
            }
        }
    }

    info!("Thread {} completed", thread_id);
    Ok(())
}

async fn send_study_files(
    thread_id: usize,
    study_uid: &str,
    files: Vec<DicomFile>,
    args: &Args,
    results: &Arc<Mutex<Vec<TransferResult>>>,
    stats: &Arc<TransferStats>,
    progress: &ProgressBar,
) -> Result<()> {
    info!("Opening association for study: {}", study_uid);

    let client = DicomClient::new(
        args.calling_ae.clone(),
        args.ae_title.clone(),
        args.host.clone(),
        args.port,
    );

    // Send all files in this study using a single association
    match client.send_files(&files).await {
        Ok(file_results) => {
            for (file, file_result) in files.iter().zip(file_results.iter()) {
                let result = TransferResult {
                    file_path: file.path.to_string_lossy().to_string(),
                    study_instance_uid: file.study_instance_uid.clone(),
                    sop_instance_uid: file.sop_instance_uid.clone(),
                    success: file_result.success,
                    error_message: file_result.error_message.clone(),
                    transfer_time_ms: file_result.transfer_time.as_millis() as u64,
                    file_size: file.file_size,
                    timestamp: Utc::now(),
                    thread_id,
                };

                if file_result.success {
                    stats.increment_success(file.file_size);
                } else {
                    stats.increment_failure();
                }

                {
                    let mut results_guard = results.lock().unwrap();
                    results_guard.push(result);
                }

                progress.inc(1);
                
                info!("Thread {} sent file: {} ({}ms)", 
                      thread_id, 
                      file.path.display(), 
                      file_result.transfer_time.as_millis());
            }
        }
        Err(e) => {
            error!("Failed to send study {}: {}", study_uid, e);
            // Mark all files as failed
            for file in &files {
                let result = TransferResult {
                    file_path: file.path.to_string_lossy().to_string(),
                    study_instance_uid: file.study_instance_uid.clone(),
                    sop_instance_uid: file.sop_instance_uid.clone(),
                    success: false,
                    error_message: Some(e.to_string()),
                    transfer_time_ms: 0,
                    file_size: file.file_size,
                    timestamp: Utc::now(),
                    thread_id,
                };

                stats.increment_failure();

                {
                    let mut results_guard = results.lock().unwrap();
                    results_guard.push(result);
                }

                progress.inc(1);
            }
        }
    }

    Ok(())
}

// Remove the simulate_transfer function since we're using real DICOM now
