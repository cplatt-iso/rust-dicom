#[path = "../common/mod.rs"]
mod common;

use anyhow::{Context, Result};
use chrono::Utc;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::Semaphore;
use tracing::{debug, error, info, warn};

use dicom_ul::association::server::{AcceptorBuilder, AssociationAcceptor};
use dicom_ul::pdu::{PDataValue, PresentationDataValue};
use dicom_ul::association::ServerAssociation;

use common::sop_classes::{SopClassRegistry, get_default_transfer_syntaxes};
use common::transfer_syntaxes::TransferSyntaxRegistry;

#[derive(Debug, Clone)]
pub struct DicomReceiver {
    ae_title: String,
    output_dir: PathBuf,
    sop_registry: Arc<SopClassRegistry>,
    transfer_registry: Arc<TransferSyntaxRegistry>,
    connection_semaphore: Arc<Semaphore>,
}

impl DicomReceiver {
    pub fn new(ae_title: String, output_dir: PathBuf, max_connections: usize) -> Self {
        // Create output directory if it doesn't exist
        if let Err(e) = std::fs::create_dir_all(&output_dir) {
            error!("Failed to create output directory {}: {}", output_dir.display(), e);
        }

        Self {
            ae_title,
            output_dir,
            sop_registry: Arc::new(SopClassRegistry::new()),
            transfer_registry: Arc::new(TransferSyntaxRegistry::new()),
            connection_semaphore: Arc::new(Semaphore::new(max_connections)),
        }
    }

    pub async fn start(&self, port: u16) -> Result<()> {
        info!("📥  DICOM receiver listening on port {}", port);
        println!("📥  DICOM receiver listening on port {}", port);

        // Create the acceptor builder with our AE title
        let mut acceptor_builder = AcceptorBuilder::new()
            .ae_title(&self.ae_title);

        // Register all supported SOP classes with their transfer syntaxes
        let supported_transfer_syntaxes = get_default_transfer_syntaxes();
        
        for sop_class_uid in self.sop_registry.get_all_uids() {
            for transfer_syntax in &supported_transfer_syntaxes {
                acceptor_builder = acceptor_builder
                    .presentation_context(sop_class_uid, transfer_syntax);
            }
        }

        // Build the acceptor
        let acceptor = acceptor_builder.build();

        // Start listening for connections
        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
        
        info!("✅  DICOM receiver ready to accept connections");
        println!("✅  DICOM receiver ready to accept connections");

        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    info!("🔗  New connection from {}", addr);
                    println!("🔗  New connection from {}", addr);
                    
                    let receiver = self.clone();
                    let acceptor = acceptor.clone();
                    
                    tokio::spawn(async move {
                        if let Err(e) = receiver.handle_connection(stream, acceptor, addr).await {
                            error!("❌  Error handling connection from {}: {}", addr, e);
                            println!("❌  Error handling connection from {}: {}", addr, e);
                        }
                    });
                }
                Err(e) => {
                    error!("❌  Failed to accept connection: {}", e);
                    println!("❌  Failed to accept connection: {}", e);
                }
            }
        }
    }

    async fn handle_connection(
        &self, 
        stream: tokio::net::TcpStream, 
        acceptor: AssociationAcceptor,
        addr: std::net::SocketAddr
    ) -> Result<()> {
        // Acquire semaphore permit for connection limiting
        let _permit = self.connection_semaphore.acquire().await?;

        info!("🔄  Handling connection from {}", addr);
        
        // Accept the association
        let mut association = acceptor.accept(stream).await
            .context("Failed to accept DICOM association")?;

        info!("✅  Association established with {}", addr);
        println!("✅  Association established with {}", addr);

        // Handle incoming requests
        loop {
            match association.receive().await {
                Ok(pdu) => {
                    match pdu {
                        dicom_ul::pdu::Pdu::PData { data } => {
                            self.handle_pdata(&data, &mut association).await?;
                        }
                        dicom_ul::pdu::Pdu::AReleaseRQ => {
                            info!("📤  Received release request from {}", addr);
                            println!("📤  Received release request from {}", addr);
                            association.send(&dicom_ul::pdu::Pdu::AReleaseRP).await?;
                            info!("✅  Sent release response to {}", addr);
                            println!("✅  Sent release response to {}", addr);
                            break;
                        }
                        _ => {
                            debug!("Received other PDU type: {:?}", pdu);
                        }
                    }
                }
                Err(e) => {
                    info!("🔌  Connection closed by peer {}: {}", addr, e);
                    println!("🔌  Connection closed by peer {}: {}", addr, e);
                    break;
                }
            }
        }

        info!("📡  Association closed with {}", addr);
        println!("📡  Association closed with {}", addr);
        Ok(())
    }

    async fn handle_pdata(
        &self, 
        data: &[PDataValue], 
        association: &mut ServerAssociation
    ) -> Result<()> {
        for pdata_value in data {
            match &pdata_value.data {
                PresentationDataValue::Command(command_data) => {
                    debug!("📝  Received command data: {} bytes", command_data.len());
                    // For now, just acknowledge with success
                    // In a full implementation, we would parse the DIMSE command
                }
                PresentationDataValue::Data(dataset_data) => {
                    info!("📥  Received dataset: {} bytes", dataset_data.len());
                    println!("📥  Received dataset: {} bytes", dataset_data.len());
                    
                    // Save the dataset to file
                    let filename = format!("received_{}.dcm", Utc::now().format("%Y%m%d_%H%M%S_%f"));
                    let file_path = self.output_dir.join(filename);
                    
                    fs::write(&file_path, dataset_data).await?;
                    info!("✅  Saved dataset to {}", file_path.display());
                    println!("✅  Saved dataset to {}", file_path.display());
                }
            }
        }

        // Send a C-STORE response (simplified - normally we'd parse the command first)
        let response_pdu = dicom_ul::pdu::Pdu::PData {
            data: vec![PDataValue {
                presentation_context_id: 1, // This should match the request context
                is_last: true,
                is_command: true,
                data: PresentationDataValue::Command(vec![0x00; 64]), // Simplified response
            }]
        };
        
        association.send(&response_pdu).await?;
        info!("✅  Sent C-STORE response");
        println!("✅  Sent C-STORE response");
        
        Ok(())
    }
}
