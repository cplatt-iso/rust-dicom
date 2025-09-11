#[path = "../common/mod.rs"]
mod common;

use anyhow::{Context, Result};
use chrono::Utc;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::Semaphore;
use tracing::{debug, error, info};

use dicom_ul::association::server::ServerAssociationOptions;
use dicom_ul::pdu::{Pdu, PDataValue, PDataValueType};

use common::sop_classes::SopClassRegistry;
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

    pub async fn start(self: Arc<Self>, port: u16) -> Result<()> {
        info!("📥  DICOM receiver listening on port {}", port);
        println!("📥  DICOM receiver listening on port {}", port);

        // Create server association options using shared/common SOP classes
        let mut server_options = ServerAssociationOptions::new()
            .accept_called_ae_title()
            .ae_title(&self.ae_title);

        // Register all supported SOP classes from our shared registry
        for sop_class_uid in self.sop_registry.get_all_uids() {
            server_options = server_options.with_abstract_syntax(sop_class_uid);
        }

        // Start listening for connections
        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
        
        info!("✅  DICOM receiver ready to accept connections");
        println!("✅  DICOM receiver ready to accept connections");

        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    info!("🔗  New connection from {}", addr);
                    println!("🔗  New connection from {}", addr);
                    
                    let receiver = Arc::clone(&self);
                    let server_options = server_options.clone();
                    
                    tokio::task::spawn_blocking(move || {
                        if let Err(e) = Self::handle_connection_blocking(receiver, stream, server_options, addr) {
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

    fn handle_connection_blocking(
        receiver: Arc<Self>, 
        stream: tokio::net::TcpStream, 
        server_options: ServerAssociationOptions<'_, dicom_ul::association::server::AcceptCalledAeTitle>,
        addr: std::net::SocketAddr
    ) -> Result<()> {
        let rt = tokio::runtime::Handle::current();
        
        rt.block_on(async {
            // Acquire semaphore permit for connection limiting
            let _permit = receiver.connection_semaphore.acquire().await?;

            info!("🔄  Handling connection from {}", addr);
            
            // Convert tokio stream to std stream for establish
            let std_stream = stream.into_std()?;
            
            // Establish the association using the server options
            let mut association = server_options.establish(std_stream)
                .context("Failed to establish DICOM association")?;

            info!("✅  Association established with {}", addr);
            println!("✅  Association established with {}", addr);

            // Log the accepted presentation contexts
            for pc in association.presentation_contexts() {
                info!("📋  Accepted presentation context {} with transfer syntax {}", pc.id, pc.transfer_syntax);
                println!("📋  Accepted presentation context {} with transfer syntax {}", pc.id, pc.transfer_syntax);
            }

            // Handle incoming requests
            loop {
                match association.receive() {
                    Ok(pdu) => {
                        match pdu {
                            Pdu::PData { data } => {
                                receiver.handle_pdata(&data).await?;
                            }
                            Pdu::ReleaseRQ => {
                                info!("📤  Received release request from {}", addr);
                                println!("📤  Received release request from {}", addr);
                                association.send(&Pdu::ReleaseRP)?;
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
            
            Ok::<(), anyhow::Error>(())
        })
    }

    async fn handle_pdata(&self, data: &[PDataValue]) -> Result<()> {
        for pdata_value in data {
            match pdata_value.value_type {
                PDataValueType::Command => {
                    debug!("📝  Received command data: {} bytes", pdata_value.data.len());
                    // For now, just acknowledge with success
                    // In a full implementation, we would parse the DIMSE command
                }
                PDataValueType::Data => {
                    info!("📥  Received dataset: {} bytes", pdata_value.data.len());
                    println!("📥  Received dataset: {} bytes", pdata_value.data.len());
                    
                    // Save the dataset to file
                    let filename = format!("received_{}.dcm", Utc::now().format("%Y%m%d_%H%M%S_%f"));
                    let file_path = self.output_dir.join(filename);
                    
                    fs::write(&file_path, &pdata_value.data).await?;
                    info!("✅  Saved dataset to {}", file_path.display());
                    println!("✅  Saved dataset to {}", file_path.display());
                }
            }
        }

        // TODO: Send proper C-STORE response
        // For now, we'll just handle the data without responding
        info!("📦  Processed P-DATA");
        println!("📦  Processed P-DATA");
        
        Ok(())
    }
}
