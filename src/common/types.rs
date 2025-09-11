use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DicomFile {
    pub path: PathBuf,
    pub study_instance_uid: String,
    pub series_instance_uid: String,
    pub sop_instance_uid: String,
    pub sop_class_uid: String,
    pub file_size: u64,
    pub modality: Option<String>,
    pub patient_id: Option<String>,
    pub study_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferResult {
    pub file_path: String,
    pub study_instance_uid: String,
    pub sop_instance_uid: String,
    pub success: bool,
    pub error_message: Option<String>,
    pub transfer_time_ms: u64,
    pub file_size: u64,
    pub timestamp: DateTime<Utc>,
    pub thread_id: usize,
}

#[derive(Debug)]
pub struct TransferStats {
    pub total_files: usize,
    pub successful_transfers: usize,
    pub failed_transfers: usize,
    pub total_bytes: u64,
    pub total_time: Duration,
    pub transfer_times: Vec<Duration>,
}

impl TransferStats {
    pub fn new() -> Self {
        Self {
            total_files: 0,
            successful_transfers: 0,
            failed_transfers: 0,
            total_bytes: 0,
            total_time: Duration::from_secs(0),
            transfer_times: Vec::new(),
        }
    }

    pub fn get_throughput_mbps(&self) -> f64 {
        let elapsed = self.total_time.as_secs_f64();
        let bytes = self.total_bytes as f64;
        if elapsed > 0.0 {
            (bytes / (1024.0 * 1024.0)) / elapsed
        } else {
            0.0
        }
    }

    pub fn get_average_transfer_time_ms(&self) -> f64 {
        if self.transfer_times.is_empty() {
            0.0
        } else {
            let total_ms: u64 = self.transfer_times.iter()
                .map(|d| d.as_millis() as u64)
                .sum();
            total_ms as f64 / self.transfer_times.len() as f64
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionSummary {
    pub session_id: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub total_files: usize,
    pub successful_transfers: usize,
    pub failed_transfers: usize,
    pub total_bytes: u64,
    pub total_time_ms: u64,
    pub average_transfer_time_ms: f64,
    pub throughput_mbps: f64,
    pub threads_used: usize,
    pub destination: String,
    pub calling_ae: String,
    pub called_ae: String,
    pub studies_processed: Vec<String>,
}
