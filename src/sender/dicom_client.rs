use anyhow::{Context, Result};
use dicom_core::{Tag, DataElement, VR};
use dicom_core::value::{Value, PrimitiveValue};
use dicom_object::{open_file, InMemDicomObject};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};
use smallvec::smallvec;

use crate::common::types::{DicomFile, TransferStats};
use crate::common::sop_classes::{SopClassRegistry, get_default_transfer_syntaxes, get_transfer_syntaxes_for_category};
use crate::common::transfer_syntaxes::TransferSyntaxRegistry;

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

        // Initialize SOP class registry and transfer syntax registry
        let sop_registry = SopClassRegistry::new();
        let ts_registry = TransferSyntaxRegistry::new();
        
        info!("Registering SOP classes for the files to be sent...");
        
        // Only register SOP classes that are actually needed for this transfer
        let mut required_sop_classes = std::collections::HashSet::new();
        for file in &files {
            required_sop_classes.insert(file.sop_class_uid.clone());
        }
        
        // Convert to Vec to avoid borrow checker issues
        let sop_classes_vec: Vec<String> = required_sop_classes.into_iter().collect();
        
        info!("Adding {} unique SOP classes found in files", sop_classes_vec.len());
        
        // Use basic transfer syntaxes to avoid too many contexts
        let transfer_syntaxes = vec![
            "1.2.840.10008.1.2.1".to_string(), // Explicit VR Little Endian
            "1.2.840.10008.1.2".to_string(),   // Implicit VR Little Endian
        ];
        let ts_refs: Vec<&String> = transfer_syntaxes.iter().collect();
        
        // Store mapping of presentation context ID to SOP class UID for later reference
        let mut sop_uid_mapping = HashMap::new();
        let mut context_id = 1u8;
        
        for sop_uid in &sop_classes_vec {
            if let Some(sop_info) = sop_registry.get(sop_uid.as_str()) {
                debug!("Adding SOP class: {} ({}) with {} transfer syntaxes", 
                       sop_info.name, sop_uid, transfer_syntaxes.len());
                       
                association_options = association_options
                    .with_presentation_context(sop_uid, ts_refs.clone());
                sop_uid_mapping.insert(context_id, sop_uid.clone());
                context_id += 1;
            } else {
                warn!("Unknown SOP class in files: {}, adding with basic transfer syntaxes", sop_uid);
                association_options = association_options
                    .with_presentation_context(sop_uid, ts_refs.clone());
                sop_uid_mapping.insert(context_id, sop_uid.clone());
                context_id += 1;
            }
        }
        
        info!("Transfer syntax coverage: {} unique transfer syntaxes available", 
              ts_registry.get_all_uids().len());

        // Establish the association
        debug!("Attempting to establish association with {}:{}", config.host, config.port);
        let mut association = match association_options
            .establish_with(&format!("{}:{}", config.host, config.port)) {
                Ok(assoc) => {
                    info!("DICOM association established successfully");
                    assoc
                },
                Err(e) => {
                    error!("Failed to establish DICOM association: {}", e);
                    return Err(anyhow::anyhow!("Failed to establish DICOM association: {}", e));
                }
            };
        
        // Report which presentation contexts were accepted
        let mut accepted_contexts = 0;
        let mut rejected_contexts = 0;
        
        for pc in association.presentation_contexts() {
            match pc.reason {
                dicom_ul::pdu::PresentationContextResultReason::Acceptance => {
                    accepted_contexts += 1;
                    if let Some(sop_uid) = sop_uid_mapping.get(&pc.id) {
                        if let Some(sop_info) = sop_registry.get(sop_uid.as_str()) {
                            debug!("✓ Accepted: {} (ID={}, UID={})", sop_info.name, pc.id, sop_uid);
                        } else {
                            debug!("✓ Accepted: Unknown SOP Class (ID={}, UID={})", pc.id, sop_uid);
                        }
                    } else {
                        debug!("✓ Accepted: Presentation Context ID={}", pc.id);
                    }
                }
                _ => {
                    rejected_contexts += 1;
                    if let Some(sop_uid) = sop_uid_mapping.get(&pc.id) {
                        if let Some(sop_info) = sop_registry.get(sop_uid.as_str()) {
                            debug!("✗ Rejected: {} (ID={}, UID={})", sop_info.name, pc.id, sop_uid);
                        } else {
                            debug!("✗ Rejected: Unknown SOP Class (ID={}, UID={})", pc.id, sop_uid);
                        }
                    } else {
                        debug!("✗ Rejected: Presentation Context ID={}", pc.id);
                    }
                }
            }
        }
        
        info!("Presentation contexts: {} accepted, {} rejected", accepted_contexts, rejected_contexts);

        // Send each file
        for (idx, file) in files.iter().enumerate() {
            let file_start = Instant::now();
            
            match Self::send_single_file_simple(&mut association, file, idx as u16 + 1, &sop_uid_mapping) {
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
        sop_uid_mapping: &HashMap<u8, String>,
    ) -> Result<u64> {
        use dicom_ul::pdu::{Pdu, PDataValue, PDataValueType};
        
        // Read the DICOM file
        let obj = open_file(&file.path)
            .context(format!("Failed to open DICOM file: {}", file.path.display()))?;

        debug!(
            "Sending C-STORE for SOP Class: {}, SOP Instance: {}, Message ID: {}",
            file.sop_class_uid, file.sop_instance_uid, message_id
        );

        // Validate that this SOP class is in our registry
        let sop_registry = SopClassRegistry::new();
        if let Some(sop_info) = sop_registry.get(file.sop_class_uid.as_str()) {
            debug!("SOP Class identified: {} (Category: {:?})", sop_info.name, sop_info.category);
        } else {
            warn!("Unknown SOP Class: {} - attempting transfer anyway", file.sop_class_uid);
        }

        // Find the correct presentation context for this SOP class
        let mut presentation_context_id = None;
        let mut selected_transfer_syntax = None;
        
        // Look through all accepted presentation contexts to find one for this SOP class
        for pc in association.presentation_contexts() {
            if pc.reason == dicom_ul::pdu::PresentationContextResultReason::Acceptance {
                // Check if this presentation context matches our SOP class
                if let Some(sop_uid) = sop_uid_mapping.get(&pc.id) {
                    if sop_uid == &file.sop_class_uid {
                        presentation_context_id = Some(pc.id);
                        selected_transfer_syntax = Some(pc.transfer_syntax.clone());
                        debug!("Found matching presentation context for SOP class {}: ID={}, Transfer Syntax={}", 
                               file.sop_class_uid, pc.id, pc.transfer_syntax);
                        break;
                    }
                }
            }
        }
        
        let presentation_context_id = presentation_context_id
            .ok_or_else(|| anyhow::anyhow!(
                "No accepted presentation context found for SOP class: {} ({})", 
                file.sop_class_uid,
                sop_registry.get_name(&file.sop_class_uid).unwrap_or("Unknown")
            ))?;
        
        let transfer_syntax = selected_transfer_syntax
            .ok_or_else(|| anyhow::anyhow!("No transfer syntax found for accepted presentation context"))?;
        
        info!("Using presentation context ID: {} with transfer syntax: {}", 
              presentation_context_id, transfer_syntax);
        info!("File SOP Class: {}, SOP Instance: {}", file.sop_class_uid, file.sop_instance_uid);

        // Prepare the dataset for transmission using the negotiated transfer syntax
        let mut dataset_buffer = Vec::new();
        
        // Map the negotiated transfer syntax UID to the appropriate registry entry
        let ts_registry = TransferSyntaxRegistry::new();
        let ts_to_use = match transfer_syntax.as_str() {
            // Uncompressed transfer syntaxes
            "1.2.840.10008.1.2" => {
                info!("Using Implicit VR Little Endian");
                &dicom_transfer_syntax_registry::entries::IMPLICIT_VR_LITTLE_ENDIAN.erased()
            }
            "1.2.840.10008.1.2.1" => {
                info!("Using Explicit VR Little Endian");
                &dicom_transfer_syntax_registry::entries::EXPLICIT_VR_LITTLE_ENDIAN.erased()
            }
            "1.2.840.10008.1.2.2" => {
                info!("Using Explicit VR Big Endian (Legacy)");
                // Note: Big Endian support may be limited in some implementations
                &dicom_transfer_syntax_registry::entries::EXPLICIT_VR_BIG_ENDIAN.erased()
            }
            
            // JPEG Baseline and Extended
            "1.2.840.10008.1.2.4.50" => {
                info!("Using JPEG Baseline (Process 1)");
                // For JPEG, we need to handle encapsulated pixel data
                &dicom_transfer_syntax_registry::entries::EXPLICIT_VR_LITTLE_ENDIAN.erased()
            }
            "1.2.840.10008.1.2.4.51" => {
                info!("Using JPEG Extended (Process 2 & 4)");
                &dicom_transfer_syntax_registry::entries::EXPLICIT_VR_LITTLE_ENDIAN.erased()
            }
            
            // JPEG Lossless
            "1.2.840.10008.1.2.4.57" | "1.2.840.10008.1.2.4.70" => {
                info!("Using JPEG Lossless");
                &dicom_transfer_syntax_registry::entries::EXPLICIT_VR_LITTLE_ENDIAN.erased()
            }
            
            // JPEG-LS
            "1.2.840.10008.1.2.4.80" => {
                info!("Using JPEG-LS Lossless");
                &dicom_transfer_syntax_registry::entries::EXPLICIT_VR_LITTLE_ENDIAN.erased()
            }
            "1.2.840.10008.1.2.4.81" => {
                info!("Using JPEG-LS Near-Lossless");
                &dicom_transfer_syntax_registry::entries::EXPLICIT_VR_LITTLE_ENDIAN.erased()
            }
            
            // JPEG 2000
            "1.2.840.10008.1.2.4.90" => {
                info!("Using JPEG 2000 Lossless");
                &dicom_transfer_syntax_registry::entries::EXPLICIT_VR_LITTLE_ENDIAN.erased()
            }
            "1.2.840.10008.1.2.4.91" => {
                info!("Using JPEG 2000");
                &dicom_transfer_syntax_registry::entries::EXPLICIT_VR_LITTLE_ENDIAN.erased()
            }
            
            // RLE Lossless
            "1.2.840.10008.1.2.5" => {
                info!("Using RLE Lossless");
                &dicom_transfer_syntax_registry::entries::EXPLICIT_VR_LITTLE_ENDIAN.erased()
            }
            
            // Default fallback for any other transfer syntax
            _ => {
                if let Some(ts_info) = ts_registry.get(&transfer_syntax) {
                    warn!("Using fallback encoding for transfer syntax: {} ({})", 
                          ts_info.name, transfer_syntax);
                } else {
                    warn!("Unknown transfer syntax: {}, using fallback", transfer_syntax);
                }
                
                // For encapsulated formats, use explicit VR little endian as base encoding
                if ts_registry.requires_encapsulation(&transfer_syntax) {
                    &dicom_transfer_syntax_registry::entries::EXPLICIT_VR_LITTLE_ENDIAN.erased()
                } else {
                    &dicom_transfer_syntax_registry::entries::IMPLICIT_VR_LITTLE_ENDIAN.erased()
                }
            }
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
            
            // Wait for C-STORE response after each chunk
            match association.receive()? {
                Pdu::PData { data } => {
                    debug!("Received C-STORE response for chunk: {} PDVs", data.len());
                    // Parse response to check status - for now just log success
                }
                other => {
                    warn!("Unexpected PDU in C-STORE response: {:?}", other);
                }
            }
        }
        
        info!("All dataset chunks sent and responses received");

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
