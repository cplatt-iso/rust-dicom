mod dicom_client;
mod types;

use anyhow::Result;
use chrono::{DateTime, Utc};
use clap::Parser;
use console::{style, Emoji};
use dicom::object::open_file;
use dicom_core::header::Tag;
use dicom_client::{DicomClient, DicomClientConfig};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;
use tracing::{error, info, warn};
use uuid::Uuid;
use walkdir::WalkDir;

use types::{DicomFile, SessionSummary, TransferResult, TransferStats};

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

    // Step 3: Setup progress tracking
    let multi_progress = MultiProgress::new();
    let main_progress = multi_progress.add(ProgressBar::new(dicom_files.len() as u64));
    main_progress.set_style(
        ProgressStyle::default_bar()
            .template("  [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({eta})")?
            .progress_chars("#>-"),
    );

    // Step 4: Send files using multiple threads
    println!("{} Starting transfer with {} threads...", ROCKET, args.threads);
    
    let study_chunks: Vec<_> = studies.into_iter().collect();
    let chunk_size = (study_chunks.len() + args.threads - 1) / args.threads;
    
    let mut handles: Vec<JoinHandle<Result<TransferStats>>> = Vec::new();
    let mut all_results = Vec::new();

    for (thread_id, chunk) in study_chunks.chunks(chunk_size).enumerate() {
        let chunk = chunk.to_vec();
        let args = args.clone();
        let progress = main_progress.clone();

        let handle = tokio::spawn(async move {
            send_studies_worker(thread_id, chunk, &args, progress).await
        });

        handles.push(handle);
    }

    // Wait for all threads to complete and collect results
    let mut combined_stats = TransferStats::new();
    for handle in handles {
        match handle.await? {
            Ok(stats) => {
                combined_stats.total_files += stats.total_files;
                combined_stats.successful_transfers += stats.successful_transfers;
                combined_stats.failed_transfers += stats.failed_transfers;
                combined_stats.total_bytes += stats.total_bytes;
                combined_stats.transfer_times.extend(stats.transfer_times);
                if combined_stats.total_time < stats.total_time {
                    combined_stats.total_time = stats.total_time;
                }
            }
            Err(e) => {
                error!("Thread failed: {}", e);
            }
        }
    }

    main_progress.finish_with_message("Transfer completed!");

    let end_time = Utc::now();
    let duration = end_time.signed_duration_since(start_time);

    // Step 5: Generate summary
    let summary = SessionSummary {
        session_id: session_id.clone(),
        start_time,
        end_time,
        total_files: combined_stats.total_files,
        successful_transfers: combined_stats.successful_transfers,
        failed_transfers: combined_stats.failed_transfers,
        total_bytes: combined_stats.total_bytes,
        total_time_ms: duration.num_milliseconds() as u64,
        average_transfer_time_ms: combined_stats.get_average_transfer_time_ms(),
        throughput_mbps: combined_stats.get_throughput_mbps(),
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
    println!("Successful:      {}", style(summary.successful_transfers).green());
    println!("Failed:          {}", style(summary.failed_transfers).red());
    println!("Total size:      {:.2} MB", summary.total_bytes as f64 / (1024.0 * 1024.0));
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

async fn send_studies_worker(
    thread_id: usize,
    studies: Vec<(String, Vec<DicomFile>)>,
    args: &Args,
    progress: ProgressBar,
) -> Result<TransferStats> {
    let mut combined_stats = TransferStats::new();

    let client_config = DicomClientConfig {
        calling_ae: args.calling_ae.clone(),
        called_ae: args.ae_title.clone(),
        host: args.host.clone(),
        port: args.port,
        timeout: Duration::from_secs(30),
    };

    for (study_uid, files) in studies {
        info!("Thread {}: Processing study {} with {} files", 
              thread_id, study_uid, files.len());

        let client = DicomClient::new(client_config.clone());
        
        match client.send_files(files.clone()).await {
            Ok(stats) => {
                combined_stats.total_files += stats.total_files;
                combined_stats.successful_transfers += stats.successful_transfers;
                combined_stats.failed_transfers += stats.failed_transfers;
                combined_stats.total_bytes += stats.total_bytes;
                combined_stats.transfer_times.extend(stats.transfer_times);
                
                // Update progress
                progress.inc(stats.successful_transfers as u64 + stats.failed_transfers as u64);
                
                info!("Thread {}: Study {} completed - {}/{} files successful", 
                      thread_id, study_uid, stats.successful_transfers, stats.total_files);
            }
            Err(e) => {
                error!("Thread {}: Failed to send study {}: {}", thread_id, study_uid, e);
                combined_stats.failed_transfers += files.len();
                progress.inc(files.len() as u64);
            }
        }
    }

    Ok(combined_stats)
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
            let study_instance_uid = obj.element(Tag(0x0020, 0x000D))
                .map(|e| e.string().unwrap_or_default().trim().to_string())
                .unwrap_or_else(|_| "UNKNOWN_STUDY".to_string());

            let series_instance_uid = obj.element(Tag(0x0020, 0x000E))
                .map(|e| e.string().unwrap_or_default().trim().to_string())
                .unwrap_or_else(|_| "UNKNOWN_SERIES".to_string());

            let sop_instance_uid = obj.element(Tag(0x0008, 0x0018))
                .map(|e| e.string().unwrap_or_default().trim().to_string())
                .unwrap_or_else(|_| "UNKNOWN_SOP_INSTANCE".to_string());

            let sop_class_uid = obj.element(Tag(0x0008, 0x0016))
                .map(|e| e.string().unwrap_or_default().trim().to_string())
                .unwrap_or_else(|_| "UNKNOWN_SOP_CLASS".to_string());

            let modality = obj.element(Tag(0x0008, 0x0060))
                .ok()
                .and_then(|e| e.string().ok())
                .map(|s| s.trim().to_string());

            let patient_id = obj.element(Tag(0x0010, 0x0020))
                .ok()
                .and_then(|e| e.string().ok())
                .map(|s| s.trim().to_string());

            let study_date = obj.element(Tag(0x0008, 0x0020))
                .ok()
                .and_then(|e| e.string().ok())
                .map(|s| s.trim().to_string());

            let file_size = std::fs::metadata(path)?.len();

            Ok(Some(DicomFile {
                path: path.to_path_buf(),
                study_instance_uid,
                series_instance_uid,
                sop_instance_uid,
                sop_class_uid,
                file_size,
                modality,
                patient_id,
                study_date,
            }))
        }
        Err(e) => {
            warn!("Failed to read DICOM file {}: {}", path.display(), e);
            Ok(None)
        }
    }
}
