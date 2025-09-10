use anyhow::{Context, Result};
use dicom_core::Tag;
use dicom_object::open_file;
use dicom_ul::association::client::ClientAssociationOptions;
use dicom_ul::pdu::{PDataValue, Pdu};
use dicom_ul::{ClientAssociation, Presentation};
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tracing::{debug, error, info, warn};

use crate::types::{DicomFile, TransferResult, TransferStats};

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

        // Establish association
        let association = self.establish_association().await?;
        
        // Send each file
        for file in &files {
            let file_start = Instant::now();
            
            match self.send_single_file(&association, file).await {
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

        // Release association
        if let Err(e) = association.release().await {
            warn!("Failed to properly release association: {}", e);
        }

        stats.total_time = start_time.elapsed();
        stats.total_files = files.len();

        info!(
            "Association completed: {}/{} files sent successfully",
            stats.successful_transfers, stats.total_files
        );

        Ok(stats)
    }

    async fn establish_association(&self) -> Result<ClientAssociation<TcpStream>> {
        let stream = TcpStream::connect(format!("{}:{}", self.config.host, self.config.port))
            .await
            .context("Failed to connect to DICOM server")?;

        // Set up presentation contexts for common transfer syntaxes
        let presentation_contexts = vec![
            Presentation::new(
                1,
                "1.2.840.10008.5.1.4.1.1.7", // Secondary Capture Image Storage
                vec![
                    "1.2.840.10008.1.2.1", // Explicit VR Little Endian
                    "1.2.840.10008.1.2",   // Implicit VR Little Endian
                ],
            ),
            Presentation::new(
                3,
                "1.2.840.10008.5.1.4.1.1.2", // CT Image Storage
                vec![
                    "1.2.840.10008.1.2.1", // Explicit VR Little Endian
                    "1.2.840.10008.1.2",   // Implicit VR Little Endian
                ],
            ),
            Presentation::new(
                5,
                "1.2.840.10008.5.1.4.1.1.4", // MR Image Storage
                vec![
                    "1.2.840.10008.1.2.1", // Explicit VR Little Endian
                    "1.2.840.10008.1.2",   // Implicit VR Little Endian
                ],
            ),
            Presentation::new(
                7,
                "1.2.840.10008.5.1.4.1.1.1", // Computed Radiography Image Storage
                vec![
                    "1.2.840.10008.1.2.1", // Explicit VR Little Endian
                    "1.2.840.10008.1.2",   // Implicit VR Little Endian
                ],
            ),
            // Add a generic storage context
            Presentation::new(
                9,
                "1.2.840.10008.5.1.4.1.1.1.1", // Digital X-Ray Image Storage
                vec![
                    "1.2.840.10008.1.2.1", // Explicit VR Little Endian
                    "1.2.840.10008.1.2",   // Implicit VR Little Endian
                ],
            ),
        ];

        let association_options = ClientAssociationOptions::new()
            .calling_ae_title(&self.config.calling_ae)
            .called_ae_title(&self.config.called_ae)
            .with_presentation_contexts(presentation_contexts)
            .max_pdu_length(16384);

        let association = association_options
            .establish_with(stream)
            .await
            .context("Failed to establish DICOM association")?;

        debug!("Association established successfully");
        Ok(association)
    }

    async fn send_single_file(
        &self,
        association: &ClientAssociation<TcpStream>,
        file: &DicomFile,
    ) -> Result<u64> {
        // Read the DICOM file
        let obj = open_file(&file.path)
            .context(format!("Failed to open DICOM file: {}", file.path.display()))?;

        // Get SOP Class UID and SOP Instance UID
        let sop_class_uid = obj
            .element(Tag(0x0008, 0x0016))?
            .string()?
            .trim()
            .to_string();
        
        let sop_instance_uid = obj
            .element(Tag(0x0008, 0x0018))?
            .string()?
            .trim()
            .to_string();

        debug!(
            "Sending file with SOP Class UID: {}, SOP Instance UID: {}",
            sop_class_uid, sop_instance_uid
        );

        // Find appropriate presentation context
        let presentation_context_id = association
            .presentation_contexts()
            .iter()
            .find(|pc| {
                pc.abstract_syntax == sop_class_uid || 
                // Fallback to any accepted context
                pc.result == dicom_ul::pdu::PresentationContextResult::Acceptance
            })
            .map(|pc| pc.id)
            .context("No suitable presentation context found")?;

        // Convert DICOM object to bytes
        let mut buffer = Vec::new();
        obj.write_dataset_with_ts(
            &mut buffer,
            &dicom_transfer_syntax_registry::entries::IMPLICIT_VR_LITTLE_ENDIAN.erased(),
        )?;

        // Create C-STORE request
        let message_id = 1u16;
        
        // Build C-STORE command dataset
        let mut command_obj = dicom_object::InMemDicomObject::new_empty();
        command_obj.put(dicom_core::DataElement::new(
            Tag(0x0000, 0x0002), // Affected SOP Class UID
            dicom_core::VR::UI,
            dicom_core::Value::Primitive(dicom_core::PrimitiveValue::Str(sop_class_uid.clone())),
        ));
        command_obj.put(dicom_core::DataElement::new(
            Tag(0x0000, 0x0100), // Command Field
            dicom_core::VR::US,
            dicom_core::Value::Primitive(dicom_core::PrimitiveValue::U16([0x0001])), // C-STORE-RQ
        ));
        command_obj.put(dicom_core::DataElement::new(
            Tag(0x0000, 0x0110), // Message ID
            dicom_core::VR::US,
            dicom_core::Value::Primitive(dicom_core::PrimitiveValue::U16([message_id])),
        ));
        command_obj.put(dicom_core::DataElement::new(
            Tag(0x0000, 0x0700), // Priority
            dicom_core::VR::US,
            dicom_core::Value::Primitive(dicom_core::PrimitiveValue::U16([0x0000])), // Medium
        ));
        command_obj.put(dicom_core::DataElement::new(
            Tag(0x0000, 0x1000), // Affected SOP Instance UID
            dicom_core::VR::UI,
            dicom_core::Value::Primitive(dicom_core::PrimitiveValue::Str(sop_instance_uid.clone())),
        ));

        // Serialize command
        let mut command_buffer = Vec::new();
        command_obj.write_dataset_with_ts(
            &mut command_buffer,
            &dicom_transfer_syntax_registry::entries::IMPLICIT_VR_LITTLE_ENDIAN.erased(),
        )?;

        // Send command PDV
        let command_pdv = PDataValue {
            presentation_context_id,
            value_type: dicom_ul::pdu::PDataValueType::Command,
            is_last: true,
            data: command_buffer,
        };

        association.send(&Pdu::PData {
            data: vec![command_pdv],
        }).await?;

        // Send data PDV
        let data_pdv = PDataValue {
            presentation_context_id,
            value_type: dicom_ul::pdu::PDataValueType::Data,
            is_last: true,
            data: buffer.clone(),
        };

        association.send(&Pdu::PData {
            data: vec![data_pdv],
        }).await?;

        // Wait for C-STORE response
        match association.receive().await? {
            Pdu::PData { .. } => {
                debug!("Received C-STORE response");
                Ok(buffer.len() as u64)
            }
            other => {
                error!("Unexpected PDU type in response: {:?}", other);
                Err(anyhow::anyhow!("Unexpected response PDU"))
            }
        }
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
