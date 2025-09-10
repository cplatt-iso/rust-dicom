use anyhow::{Context, Result};
use dicom_core::{Tag, DataElement, VR};
use dicom_core::value::{Value, PrimitiveValue};
use dicom_object::{open_file, InMemDicomObject};
use dicom_ul::association::client::ClientAssociationOptions;
use dicom_ul::ClientAssociation;
use std::net::TcpStream;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};
use smallvec::smallvec;

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

        // Use blocking implementation - DICOM networking is synchronous
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
        let mut stats = TransferStats::new();
        
        // Connect to the DICOM server
        let stream = TcpStream::connect(format!("{}:{}", config.host, config.port))
            .context("Failed to connect to DICOM server")?;
        
        stream.set_read_timeout(Some(config.timeout))?;
        stream.set_write_timeout(Some(config.timeout))?;

        // Collect unique SOP Class UIDs from files
        let mut sop_classes = std::collections::HashSet::new();
        for file in &files {
            sop_classes.insert(file.sop_class_uid.clone());
        }

        // Build presentation contexts for all SOP classes we need
        let mut association_options = ClientAssociationOptions::new()
            .calling_ae_title(&config.calling_ae)
            .called_ae_title(&config.called_ae)
            .max_pdu_length(16384);

        // Add presentation contexts for each unique SOP class
        for sop_class in &sop_classes {
            association_options = association_options
                .with_presentation_context(
                    sop_class.as_str(),
                    vec![
                        "1.2.840.10008.1.2.1", // Explicit VR Little Endian
                        "1.2.840.10008.1.2",   // Implicit VR Little Endian
                    ],
                );
        }

        // Also add common storage SOP classes as fallbacks
        let common_sop_classes = vec![
            "1.2.840.10008.5.1.4.1.1.7",  // Secondary Capture Image Storage
            "1.2.840.10008.5.1.4.1.1.2",  // CT Image Storage
            "1.2.840.10008.5.1.4.1.1.4",  // MR Image Storage
            "1.2.840.10008.5.1.4.1.1.1",  // Computed Radiography Image Storage
        ];

        for sop_class in &common_sop_classes {
            if !sop_classes.contains(*sop_class) {
                association_options = association_options
                    .with_presentation_context(
                        *sop_class,
                        vec![
                            "1.2.840.10008.1.2.1", // Explicit VR Little Endian
                            "1.2.840.10008.1.2",   // Implicit VR Little Endian
                        ],
                    );
            }
        }

        // Establish the association
        let association = association_options
            .establish_with(&format!("{}:{}", config.host, config.port))
            .context("Failed to establish DICOM association")?;

        info!("DICOM association established successfully");
        debug!("Presentation contexts: {:?}", association.presentation_contexts());

        // Send each file
        for (idx, file) in files.iter().enumerate() {
            let file_start = Instant::now();
            
            match Self::send_single_file(&association, file, idx as u16 + 1) {
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

        // Release the association
        if let Err(e) = association.release() {
            warn!("Failed to properly release association: {}", e);
        } else {
            info!("DICOM association released successfully");
        }

        info!(
            "Transfer completed: {}/{} files sent successfully",
            stats.successful_transfers, stats.total_files
        );

        Ok(stats)
    }

    fn send_single_file(
        association: &ClientAssociation<TcpStream>,
        file: &DicomFile,
        message_id: u16,
    ) -> Result<u64> {
        // Read the DICOM file
        let obj = open_file(&file.path)
            .context(format!("Failed to open DICOM file: {}", file.path.display()))?;

        let sop_class_uid = &file.sop_class_uid;
        let sop_instance_uid = &file.sop_instance_uid;

        debug!(
            "Sending C-STORE for SOP Class: {}, SOP Instance: {}, Message ID: {}",
            sop_class_uid, sop_instance_uid, message_id
        );

        // Find appropriate presentation context
        let presentation_context = association
            .presentation_contexts()
            .iter()
            .find(|pc| pc.abstract_syntax == *sop_class_uid)
            .or_else(|| {
                // Fallback to any accepted presentation context
                association
                    .presentation_contexts()
                    .iter()
                    .find(|pc| pc.result.is_ok())
            })
            .context("No suitable presentation context found")?;

        let presentation_context_id = presentation_context.id;
        debug!("Using presentation context ID: {}", presentation_context_id);

        // Prepare the dataset for transmission
        let mut dataset_buffer = Vec::new();
        obj.write_dataset_with_ts(
            &mut dataset_buffer,
            &dicom_transfer_syntax_registry::entries::IMPLICIT_VR_LITTLE_ENDIAN.erased(),
        )?;

        // Create C-STORE command
        let mut command_obj = InMemDicomObject::new_empty();
        
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
            Value::Primitive(PrimitiveValue::U16(smallvec![0x0000])), // Medium priority
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

        // Send C-STORE request
        association
            .cstore(presentation_context_id, &command_buffer, &dataset_buffer)
            .context("Failed to send C-STORE request")?;

        debug!("C-STORE request sent successfully, {} bytes", dataset_buffer.len());
        Ok(dataset_buffer.len() as u64)
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
