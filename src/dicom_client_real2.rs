use anyhow::{Context, Result};
use dicom_core::Tag;
use dicom_object::open_file;
use std::net::TcpStream;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

use crate::types::{DicomFile, TransferStats};

#[derive(Debug, Clone)]
pub struct DicomClientConfig {
    pub calling_ae: String,
    pub called_ae: String,
    pub host: String,
    pub port: u16,
    pub timeout: Duration,
}

pub struct DicomClient {
    config: DicomClientConfig,
}

impl DicomClient {
    pub fn new(config: DicomClientConfig) -> Self {
        Self { config }
    }

    pub async fn send_files(&self, files: Vec<DicomFile>) -> Result<TransferStats> {
        let start_time = Instant::now();
        let mut stats = TransferStats::new();
        
        if files.is_empty() {
            return Ok(stats);
        }

        info!(
            "Opening association to {}@{}:{}",
            self.config.called_ae, self.config.host, self.config.port
        );

        // For now, let's use a blocking implementation but wrap it for async
        let files_clone = files.clone();
        let config = self.config.clone();
        
        let result = tokio::task::spawn_blocking(move || {
            Self::send_files_blocking(&config, files_clone)
        }).await??;

        stats.total_files = result.total_files;
        stats.successful_transfers = result.successful_transfers;
        stats.failed_transfers = result.failed_transfers;
        stats.total_bytes = result.total_bytes;
        stats.total_time = start_time.elapsed();
        stats.transfer_times = result.transfer_times;

        Ok(stats)
    }

    fn send_files_blocking(config: &DicomClientConfig, files: Vec<DicomFile>) -> Result<TransferStats> {
        use dicom_ul::association::client::ClientAssociationOptions;
        
        let mut stats = TransferStats::new();
        
        // Connect to the DICOM server
        let stream = TcpStream::connect(format!("{}:{}", config.host, config.port))
            .context("Failed to connect to DICOM server")?;
        
        stream.set_read_timeout(Some(config.timeout))?;
        stream.set_write_timeout(Some(config.timeout))?;

        // Set up presentation contexts for common SOP classes
        let mut association_options = ClientAssociationOptions::new()
            .calling_ae_title(&config.calling_ae)
            .called_ae_title(&config.called_ae)
            .max_pdu_length(16384);

        // Add presentation contexts for common SOP classes
        association_options = association_options
            .with_presentation_context(
                "1.2.840.10008.5.1.4.1.1.7", // Secondary Capture Image Storage
                vec!["1.2.840.10008.1.2.1", "1.2.840.10008.1.2"], // Transfer syntaxes
            )
            .with_presentation_context(
                "1.2.840.10008.5.1.4.1.1.2", // CT Image Storage
                vec!["1.2.840.10008.1.2.1", "1.2.840.10008.1.2"],
            )
            .with_presentation_context(
                "1.2.840.10008.5.1.4.1.1.4", // MR Image Storage
                vec!["1.2.840.10008.1.2.1", "1.2.840.10008.1.2"],
            )
            .with_presentation_context(
                "1.2.840.10008.5.1.4.1.1.1", // Computed Radiography Image Storage
                vec!["1.2.840.10008.1.2.1", "1.2.840.10008.1.2"],
            )
            .with_presentation_context(
                "1.2.840.10008.5.1.4.1.1.1.1", // Digital X-Ray Image Storage
                vec!["1.2.840.10008.1.2.1", "1.2.840.10008.1.2"],
            );

        // Establish association
        let association = association_options
            .establish_with(stream)
            .context("Failed to establish DICOM association")?;

        debug!("Association established successfully");

        // Send each file
        for file in &files {
            let file_start = Instant::now();
            
            match Self::send_single_file(&association, file) {
                Ok(bytes_sent) => {
                    let transfer_time = file_start.elapsed();
                    stats.successful_transfers += 1;
                    stats.total_bytes += bytes_sent;
                    stats.transfer_times.push(transfer_time);
                    
                    info!(
                        "✓ Sent {} ({} bytes) in {:?}",
                        file.path.display(),
                        bytes_sent,
                        transfer_time
                    );
                }
                Err(e) => {
                    stats.failed_transfers += 1;
                    error!("✗ Failed to send {}: {}", file.path.display(), e);
                }
            }
        }

        stats.total_files = files.len();

        // Release association
        if let Err(e) = association.release() {
            warn!("Failed to properly release association: {}", e);
        }

        info!(
            "Association completed: {}/{} files sent successfully",
            stats.successful_transfers, stats.total_files
        );

        Ok(stats)
    }

    fn send_single_file(
        association: &dicom_ul::ClientAssociation<TcpStream>,
        file: &DicomFile,
    ) -> Result<u64> {
        use dicom_core::value::{Value, PrimitiveValue, C};
        use dicom_core::{DataElement, VR};
        use smallvec::smallvec;
        
        // Read the DICOM file
        let obj = open_file(&file.path)
            .context(format!("Failed to open DICOM file: {}", file.path.display()))?;

        // Get SOP Class UID and SOP Instance UID from the file metadata
        let sop_class_uid = &file.sop_class_uid;
        let sop_instance_uid = &file.sop_instance_uid;

        debug!(
            "Sending file with SOP Class UID: {}, SOP Instance UID: {}",
            sop_class_uid, sop_instance_uid
        );

        // Use the first available presentation context that matches or any accepted one
        let presentation_context_id = 1u8; // Simplified - use first context

        // Convert DICOM object to bytes
        let mut buffer = Vec::new();
        obj.write_dataset_with_ts(
            &mut buffer,
            &dicom_transfer_syntax_registry::entries::IMPLICIT_VR_LITTLE_ENDIAN.erased(),
        )?;

        // Create C-STORE request using simplified approach
        let message_id = 1u16;
        
        // Build C-STORE command dataset
        let mut command_obj = dicom_object::InMemDicomObject::new_empty();
        
        // Add command elements using the correct API
        command_obj.put(DataElement::new(
            Tag(0x0000, 0x0002), // Affected SOP Class UID
            VR::UI,
            Value::Primitive(PrimitiveValue::Str(sop_class_uid.clone().into())),
        ));
        command_obj.put(DataElement::new(
            Tag(0x0000, 0x0100), // Command Field
            VR::US,
            Value::Primitive(PrimitiveValue::U16(smallvec![0x0001])), // C-STORE-RQ
        ));
        command_obj.put(DataElement::new(
            Tag(0x0000, 0x0110), // Message ID
            VR::US,
            Value::Primitive(PrimitiveValue::U16(smallvec![message_id])),
        ));
        command_obj.put(DataElement::new(
            Tag(0x0000, 0x0700), // Priority
            VR::US,
            Value::Primitive(PrimitiveValue::U16(smallvec![0x0000])), // Medium
        ));
        command_obj.put(DataElement::new(
            Tag(0x0000, 0x1000), // Affected SOP Instance UID
            VR::UI,
            Value::Primitive(PrimitiveValue::Str(sop_instance_uid.clone().into())),
        ));

        // Serialize command
        let mut command_buffer = Vec::new();
        command_obj.write_dataset_with_ts(
            &mut command_buffer,
            &dicom_transfer_syntax_registry::entries::IMPLICIT_VR_LITTLE_ENDIAN.erased(),
        )?;

        // Send C-STORE using association's store method (simplified)
        association.cstore(
            presentation_context_id,
            &command_buffer,
            &buffer,
        ).context("Failed to send C-STORE request")?;

        debug!("C-STORE completed successfully");
        Ok(buffer.len() as u64)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DicomClientError {
    #[error("Connection failed: {0}")]
    Connection(#[from] std::io::Error),
    #[error("Association failed: {0}")]
    Association(String),
    #[error("Transfer failed: {0}")]
    Transfer(String),
    #[error("DICOM parsing error: {0}")]
    DicomParsing(#[from] dicom_object::ReadError),
}
