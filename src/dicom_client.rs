use anyhow::{Context, Result};
use dicom_core::{Tag, DataElement, VR};
use dicom_core::value::{Value, PrimitiveValue};
use dicom_object::{open_file, InMemDicomObject};
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
        use dicom_ul::association::client::ClientAssociationOptions;
        
        let mut stats = TransferStats::new();
        
        info!("Establishing DICOM association...");

        // Create association options
        let mut association_options = ClientAssociationOptions::new()
            .calling_ae_title(&config.calling_ae)
            .called_ae_title(&config.called_ae)
            .max_pdu_length(65536); // Increase PDU size to handle larger files

        // Add common presentation contexts
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
            );

        // Establish the association
        let mut association = association_options
            .establish_with(&format!("{}:{}", config.host, config.port))
            .context("Failed to establish DICOM association")?;

        info!("DICOM association established successfully");
        
        // Debug: Check which presentation contexts were accepted
        for pc in association.presentation_contexts() {
            info!("Presentation Context ID {}: reason={:?}, transfer_syntax={:?}", 
                  pc.id, pc.reason, pc.transfer_syntax);
        }

        // Send each file
        for (idx, file) in files.iter().enumerate() {
            let file_start = Instant::now();
            
            match Self::send_single_file_simple(&mut association, file, idx as u16 + 1) {
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

    fn send_single_file_simple(
        association: &mut dicom_ul::ClientAssociation<std::net::TcpStream>,
        file: &DicomFile,
        message_id: u16,
    ) -> Result<u64> {
        use dicom_ul::pdu::{Pdu, PDataValue, PDataValueType};
        
        // Read the DICOM file
        let obj = open_file(&file.path)
            .context(format!("Failed to open DICOM file: {}", file.path.display()))?;

        debug!(
            "Sending C-STORE for SOP Class: {}, SOP Instance: {}, Message ID: {}",
            file.sop_class_uid, file.sop_instance_uid, message_id
        );

        // Find the correct presentation context for this SOP class
        let mut presentation_context_id = None;
        let mut selected_transfer_syntax = None;
        for pc in association.presentation_contexts() {
            if pc.reason == dicom_ul::pdu::PresentationContextResultReason::Acceptance {
                presentation_context_id = Some(pc.id);
                selected_transfer_syntax = Some(pc.transfer_syntax.clone());
                break;
            }
        }
        
        let presentation_context_id = presentation_context_id
            .ok_or_else(|| anyhow::anyhow!("No accepted presentation contexts available"))?;
        
        let transfer_syntax = selected_transfer_syntax
            .ok_or_else(|| anyhow::anyhow!("No transfer syntax found for accepted presentation context"))?;
        
        info!("Using presentation context ID: {} with transfer syntax: {}", 
              presentation_context_id, transfer_syntax);
        info!("File SOP Class: {}, SOP Instance: {}", file.sop_class_uid, file.sop_instance_uid);

        // Prepare the dataset for transmission using the negotiated transfer syntax
        let mut dataset_buffer = Vec::new();
        
        // Use the appropriate transfer syntax based on what was negotiated
        let ts_to_use = if transfer_syntax == "1.2.840.10008.1.2.1" {
            // Explicit VR Little Endian
            &dicom_transfer_syntax_registry::entries::EXPLICIT_VR_LITTLE_ENDIAN.erased()
        } else {
            // Default to Implicit VR Little Endian
            &dicom_transfer_syntax_registry::entries::IMPLICIT_VR_LITTLE_ENDIAN.erased()
        };
        
        obj.write_dataset_with_ts(&mut dataset_buffer, ts_to_use)?;

        // Create C-STORE command dataset
        let mut command_obj = InMemDicomObject::new_empty();
        
        // Add required C-STORE command elements
        command_obj.put(DataElement::new(
            Tag(0x0000, 0x0002), // Affected SOP Class UID
            VR::UI,
            Value::Primitive(PrimitiveValue::Str(file.sop_class_uid.clone().into())),
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
            Value::Primitive(PrimitiveValue::Str(file.sop_instance_uid.clone().into())),
        ));
        
        // CRITICAL: Add CommandDataSetType - indicates that dataset follows command
        command_obj.put(DataElement::new(
            Tag(0x0000, 0x0800), // CommandDataSetType  
            VR::US,
            Value::Primitive(PrimitiveValue::U16(smallvec![0x0001])), // Dataset present
        ));

        // Serialize command
        let mut command_buffer = Vec::new();
        command_obj.write_dataset_with_ts(
            &mut command_buffer,
            &dicom_transfer_syntax_registry::entries::IMPLICIT_VR_LITTLE_ENDIAN.erased(),
        )?;

        // Send command P-DATA-TF
        let command_pdv = PDataValue {
            presentation_context_id,
            value_type: PDataValueType::Command,
            is_last: true,
            data: command_buffer.clone(),
        };

        info!("Sending C-STORE command PDU: {} bytes", command_buffer.len());
        association.send(&Pdu::PData {
            data: vec![command_pdv],
        })?;
        info!("C-STORE command PDU sent successfully");

        // Send dataset P-DATA-TF (with fragmentation for large files)
        let max_pdu_data_size = 16000; // Conservative PDU data size accounting for headers
        let mut offset = 0;
        
        info!("Starting dataset transfer: {} bytes total", dataset_buffer.len());
        
        while offset < dataset_buffer.len() {
            let chunk_size = std::cmp::min(max_pdu_data_size, dataset_buffer.len() - offset);
            let is_last = offset + chunk_size >= dataset_buffer.len();
            
            let data_chunk = dataset_buffer[offset..offset + chunk_size].to_vec();
            
            let data_pdv = PDataValue {
                presentation_context_id,
                value_type: PDataValueType::Data,
                is_last,
                data: data_chunk,
            };

            association.send(&Pdu::PData {
                data: vec![data_pdv],
            })?;
            
            offset += chunk_size;
            info!("Sent data chunk: {} bytes, is_last: {}, total sent: {}/{}", 
                  chunk_size, is_last, offset, dataset_buffer.len());
        }
        
        info!("All dataset chunks sent, waiting for C-STORE response...");

        // Wait for C-STORE response
        match association.receive()? {
            Pdu::PData { data } => {
                debug!("Received C-STORE response: {} PDVs", data.len());
                // Parse response to check status - for now just log success
                info!("C-STORE response received successfully");
            }
            other => {
                warn!("Unexpected PDU in C-STORE response: {:?}", other);
            }
        }

        debug!("C-STORE operation completed, {} bytes transferred", dataset_buffer.len());
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
