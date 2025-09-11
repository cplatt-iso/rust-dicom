// Receiver binary main
mod receiver;

use anyhow::Result;
use clap::Parser;
use console::{style, Emoji};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

use receiver::DicomReceiver;

static SATELLITE: Emoji<'_, '_> = Emoji("📡 ", "");
static INBOX: Emoji<'_, '_> = Emoji("📥 ", "");

#[derive(Parser, Clone)]
#[command(name = "dicom-receiver")]
#[command(about = "A high-performance DICOM C-STORE receiver")]
#[command(version = "1.0")]
struct Args {
    /// Output directory for received DICOM files
    #[arg(short, long)]
    output: PathBuf,

    /// AE Title for this receiver
    #[arg(short = 'a', long, default_value = "RUST_SCP")]
    ae_title: String,

    /// Port to listen on
    #[arg(short, long, default_value = "4242")]
    port: u16,

    /// Maximum number of concurrent associations
    #[arg(short = 'm', long, default_value = "10")]
    max_connections: usize,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    let session_id = Uuid::new_v4().to_string();
    
    // Create logs directory if it doesn't exist
    std::fs::create_dir_all("logs")?;
    
    let log_file = format!("logs/dicom_receiver_{}.log", session_id);

    tracing_subscriber::fmt()
        .with_writer(std::fs::File::create(&log_file)?)
        .init();

    println!("{} DICOM Receiver v1.0", SATELLITE);
    println!("Session ID: {}", style(&session_id).cyan());
    println!("Log file: {}", style(&log_file).yellow());
    println!("AE Title: {}", style(&args.ae_title).green());
    println!("Port: {}", style(&args.port).green());
    println!("Output: {}", style(&args.output.display()).green());
    println!("Max connections: {}", style(&args.max_connections).green());
    println!();

    // Create output directory if it doesn't exist
    std::fs::create_dir_all(&args.output)?;

    // Start the receiver
    let receiver = Arc::new(DicomReceiver::new(
        args.ae_title.clone(),
        args.output.clone(),
        args.max_connections,
    ));

    println!("{} Starting DICOM receiver...", INBOX);
    info!("Starting DICOM receiver on port {}", args.port);

    receiver.start(args.port).await?;

    Ok(())
}
