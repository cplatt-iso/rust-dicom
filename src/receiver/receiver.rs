#[path = "../common/mod.rs"]
mod common;

use anyhow::{Context, Result};
use chrono::Utc;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::Semaphore;
use tracing::{debug, error, info};

use dicom_ul::association::server::ServerAssociationOptions;
use dicom_ul::pdu::{Pdu, PDataValue, PDataValueType};

use common::sop_classes::SopClassRegistry;
use common::transfer_syntaxes::TransferSyntaxRegistry;

#[derive(Debug)]
struct DicomTransfer {
    command_received: bool,
    dataset_chunks: Vec<Vec<u8>>,
    total_bytes: usize,
    presentation_context_id: u8,
    started_at: chrono::DateTime<Utc>,
}

impl DicomTransfer {
    fn new(presentation_context_id: u8) -> Self {
        Self {
            command_received: false,
            dataset_chunks: Vec::new(),
            total_bytes: 0,
            presentation_context_id,
            started_at: Utc::now(),
        }
    }

    fn add_chunk(&mut self, data: Vec<u8>) {
        self.total_bytes += data.len();
        self.dataset_chunks.push(data);
    }

    fn reconstruct_dataset(&self) -> Vec<u8> {
        let mut dataset = Vec::with_capacity(self.total_bytes);
        for chunk in &self.dataset_chunks {
            dataset.extend_from_slice(chunk);
        }
        dataset
    }
}

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
                    
                    tokio::task::spawn_blocking(move || {
                        if let Err(e) = Self::handle_connection_blocking(receiver, stream, addr) {
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
        addr: std::net::SocketAddr
    ) -> Result<()> {
        let rt = tokio::runtime::Handle::current();
        
        rt.block_on(async {
            // Create server association options using shared/common SOP classes
            let mut server_options = ServerAssociationOptions::new()
                .accept_called_ae_title()
                .ae_title(&receiver.ae_title)
                .promiscuous(true); // Accept unknown abstract syntaxes for maximum compatibility

            // Register all supported SOP classes from our shared registry
            for sop_class_uid in receiver.sop_registry.get_all_uids() {
                server_options = server_options.with_abstract_syntax(sop_class_uid);
            }
            
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

            // Clone receiver for use in the blocking task
            let receiver_clone = receiver.clone();
            
            // Handle incoming requests with longer timeout and more robust error handling
            let _handle_result = tokio::task::spawn_blocking(move || {
                debug!("🔄  Starting PDU receive loop...");
                println!("🔄  Starting PDU receive loop...");
                
                // Add a small delay to ensure proper connection setup
                std::thread::sleep(std::time::Duration::from_millis(100));
                
                let mut transfers: HashMap<u8, DicomTransfer> = HashMap::new();
                let mut pdu_count = 0;
                
                loop {
                    pdu_count += 1;
                    debug!("📡  Waiting for PDU #{}", pdu_count);
                    println!("📡  Waiting for PDU #{}", pdu_count);
                    
                    match association.receive() {
                        Ok(pdu) => {
                            debug!("📦  Received PDU #{}: {:?}", pdu_count, std::mem::discriminant(&pdu));
                            println!("📦  Received PDU #{}: {:?}", pdu_count, std::mem::discriminant(&pdu));
                            
                            match pdu {
                                Pdu::PData { data } => {
                                    info!("📥  Received P-DATA with {} values", data.len());
                                    println!("📥  Received P-DATA with {} values", data.len());
                                    
                                    for (i, pdata_value) in data.iter().enumerate() {
                                        println!("  PDU Value {}: {:?}, {} bytes", i+1, pdata_value.value_type, pdata_value.data.len());
                                        
                                        let pc_id = pdata_value.presentation_context_id;
                                        
                                        // Get or create transfer for this presentation context
                                        let transfer = transfers.entry(pc_id).or_insert_with(|| DicomTransfer::new(pc_id));
                                        
                                        match pdata_value.value_type {
                                            PDataValueType::Command => {
                                                debug!("📝  Received command data: {} bytes", pdata_value.data.len());
                                                println!("📝  Command PDU: {} bytes", pdata_value.data.len());
                                                transfer.command_received = true;
                                            }
                                            PDataValueType::Data => {
                                                info!("📦  Received dataset chunk: {} bytes", pdata_value.data.len());
                                                println!("📦  Dataset chunk: {} bytes", pdata_value.data.len());
                                                
                                                // Add this chunk to the transfer
                                                transfer.add_chunk(pdata_value.data.clone());
                                                
                                                // If this is the last chunk (is_last flag), reconstruct the file
                                                if pdata_value.is_last {
                                                    let complete_dataset = transfer.reconstruct_dataset();
                                                    info!("✅  Completed dataset reconstruction: {} bytes from {} chunks", 
                                                          complete_dataset.len(), transfer.dataset_chunks.len());
                                                    println!("✅  Completed dataset: {} bytes from {} chunks", 
                                                             complete_dataset.len(), transfer.dataset_chunks.len());
                                                    
                                                    // Save the complete reconstructed DICOM file
                                                    let filename = format!("received_{}_{}.dcm", 
                                                                          transfer.started_at.format("%Y%m%d_%H%M%S_%f"),
                                                                          pc_id);
                                                    let file_path = receiver_clone.output_dir.join(filename);
                                                    
                                                    if let Err(e) = std::fs::write(&file_path, &complete_dataset) {
                                                        error!("❌  Failed to save complete dataset: {}", e);
                                                        println!("❌  Failed to save complete dataset: {}", e);
                                                    } else {
                                                        info!("✅  Saved complete DICOM file to {}", file_path.display());
                                                        println!("✅  Saved complete DICOM file to {}", file_path.display());
                                                    }
                                                    
                                                    // Clean up this transfer
                                                    transfers.remove(&pc_id);
                                                }
                                            }
                                        }
                                    }
                                    
                                    // Send a simple C-STORE response after receiving any P-DATA
                                    if let Err(e) = receiver_clone.send_c_store_response(&mut association, &data) {
                                        error!("❌  Failed to send C-STORE response: {}", e);
                                        println!("❌  Failed to send C-STORE response: {}", e);
                                    } else {
                                        info!("✅  Sent C-STORE response");
                                        println!("✅  Sent C-STORE response");
                                    }
                                }
                                Pdu::ReleaseRQ => {
                                    info!("📤  Received release request from {}", addr);
                                    println!("📤  Received release request from {}", addr);
                                    if let Err(e) = association.send(&Pdu::ReleaseRP) {
                                        error!("❌  Failed to send release response: {}", e);
                                    } else {
                                        info!("✅  Sent release response to {}", addr);
                                        println!("✅  Sent release response to {}", addr);
                                    }
                                    break;
                                }
                                _ => {
                                    debug!("Received other PDU type: {:?}", pdu);
                                }
                            }
                        }
                        Err(e) => {
                            error!("❌  Error receiving PDU: {}", e);
                            println!("❌  Error receiving PDU: {}", e);
                            
                            // Log the error type for debugging
                            debug!("Error type: {:?}", e);
                            
                            // Handle common error cases
                            let error_string = e.to_string();
                            if error_string.contains("EOF") || error_string.contains("UnexpectedEof") {
                                info!("🔌  Connection closed by peer (EOF)");
                                println!("🔌  Connection closed by peer (EOF)");
                            } else if error_string.contains("Connection") {
                                info!("🔌  Connection error from peer");
                                println!("🔌  Connection error from peer");
                            } else {
                                error!("🔌  Unknown error: {}", e);
                                println!("🔌  Unknown error: {}", e);
                            }
                            
                            // Save any pending transfers before closing
                            for (pc_id, transfer) in transfers.iter() {
                                if !transfer.dataset_chunks.is_empty() {
                                    let complete_dataset = transfer.reconstruct_dataset();
                                    info!("💾  Saving pending transfer: {} bytes from {} chunks", 
                                          complete_dataset.len(), transfer.dataset_chunks.len());
                                    println!("💾  Saving pending transfer: {} bytes from {} chunks", 
                                             complete_dataset.len(), transfer.dataset_chunks.len());
                                    
                                    // Save the complete reconstructed DICOM file
                                    let filename = format!("received_{}_{}.dcm", 
                                                          transfer.started_at.format("%Y%m%d_%H%M%S_%f"),
                                                          pc_id);
                                    let file_path = receiver_clone.output_dir.join(filename);
                                    
                                    if let Err(e) = std::fs::write(&file_path, &complete_dataset) {
                                        error!("❌  Failed to save pending dataset: {}", e);
                                        println!("❌  Failed to save pending dataset: {}", e);
                                    } else {
                                        info!("✅  Saved pending DICOM file to {}", file_path.display());
                                        println!("✅  Saved pending DICOM file to {}", file_path.display());
                                    }
                                }
                            }
                            
                            break;
                        }
                    }
                }
                Ok::<(), anyhow::Error>(())
            }).await??;

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

    fn send_c_store_response(&self, association: &mut dicom_ul::association::ServerAssociation<std::net::TcpStream>, data: &[PDataValue]) -> Result<()> {
        // Extract presentation context ID from the request
        let pc_id = data.first().map(|pv| pv.presentation_context_id).unwrap_or(1);
        
        // Create a proper C-STORE response with DICOM status
        // This is a minimal DIMSE C-STORE response indicating success
        let response_data = vec![
            // Group 0000 (Command Group)
            0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x38, 0x00, 0x00, 0x00, // Command Group Length (0000,0000) = 56 bytes
            0x00, 0x00, 0x02, 0x00, 0x12, 0x00, 0x00, 0x00, 0x01, 0x80, 0x00, 0x00, // Affected SOP Class UID (0000,0002)
            0x00, 0x00, 0x00, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, // Command Field (0000,0100) = C-STORE-RSP (0x8001)
            0x00, 0x00, 0x10, 0x01, 0x02, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, // Message ID Being Responded To (0000,0120) = 1
            0x00, 0x00, 0x00, 0x09, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Status (0000,0900) = Success (0x0000)
        ];

        let response_pdu = Pdu::PData {
            data: vec![PDataValue {
                presentation_context_id: pc_id,
                is_last: true,
                value_type: PDataValueType::Command,
                data: response_data,
            }]
        };

        association.send(&response_pdu)?;
        debug!("📤  Sent C-STORE response for presentation context {}", pc_id);
        Ok(())
    }
}
