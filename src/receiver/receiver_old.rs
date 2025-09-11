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

#[derive(Debug)]
struct AssociationContext {
    calling_ae: String,
    called_ae: String,
    presentation_contexts: Vec<PresentationContext>,
    max_pdu_length: u32,
}

#[derive(Debug, Clone)]
struct PresentationContext {
    id: u8,
    abstract_syntax: String,
    transfer_syntaxes: Vec<String>,
    result: u8, // 0 = acceptance, 1 = user rejection, 2 = no reason, 3 = abstract syntax not supported, 4 = transfer syntaxes not supported
}

impl DicomReceiver {
    pub fn new(ae_title: String, output_dir: PathBuf, max_connections: usize) -> Self {
        Self {
            ae_title,
            output_dir,
            sop_registry: Arc::new(SopClassRegistry::new()),
            transfer_registry: Arc::new(TransferSyntaxRegistry::new()),
            connection_semaphore: Arc::new(Semaphore::new(max_connections)),
        }
    }

    pub async fn start(&self, port: u16) -> Result<()> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port))
            .await
            .context("Failed to bind to port")?;

        info!("DICOM receiver listening on port {}", port);

        loop {
            match listener.accept().await {
                Ok((socket, addr)) => {
                    let receiver = self.clone();
                    tokio::spawn(async move {
                        if let Err(e) = receiver.handle_connection(socket, addr).await {
                            error!("Error handling connection from {}: {}", addr, e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    }

    async fn handle_connection(&self, mut socket: TcpStream, addr: SocketAddr) -> Result<()> {
        // Acquire semaphore permit for connection limiting
        let _permit = self.connection_semaphore
            .acquire()
            .await
            .context("Failed to acquire connection permit")?;

        info!("New connection from {}", addr);

        // Handle DICOM association
        let association = self.handle_association_request(&mut socket).await?;
        
        if let Some(ctx) = association {
            info!("Association established with {} (calling: {})", addr, ctx.calling_ae);
            
            // Handle C-STORE requests
            if let Err(e) = self.handle_c_store_requests(&mut socket, &ctx).await {
                error!("Error handling C-STORE requests: {}", e);
            }
            
            info!("Association closed with {}", addr);
        } else {
            warn!("Association rejected from {}", addr);
        }

        Ok(())
    }

    async fn handle_association_request(&self, socket: &mut TcpStream) -> Result<Option<AssociationContext>> {
        // Read A-ASSOCIATE-RQ PDU header (6 bytes: type + reserved + length)
        let mut pdu_header = [0u8; 6];
        socket.read_exact(&mut pdu_header).await?;

        if pdu_header[0] != 0x01 {
            warn!("Invalid PDU type: expected A-ASSOCIATE-RQ (0x01), got 0x{:02x}", pdu_header[0]);
            return Ok(None);
        }

        // PDU length is in bytes 2-5 (4 bytes, big endian)
        let pdu_length = u32::from_be_bytes([pdu_header[2], pdu_header[3], pdu_header[4], pdu_header[5]]) as usize;
        debug!("PDU length: {}", pdu_length);
        
        let mut pdu_data = vec![0u8; pdu_length];
        socket.read_exact(&mut pdu_data).await?;
        debug!("Read PDU data of {} bytes", pdu_data.len());

        // Parse association request
        let (calling_ae, called_ae, presentation_contexts) = match self.parse_association_request(&pdu_data) {
            Ok(result) => result,
            Err(e) => {
                error!("Failed to parse association request: {}", e);
                self.send_association_reject(socket, 1, 1, 2).await?; // Protocol error
                return Ok(None);
            }
        };

        // Validate called AE
        if called_ae != self.ae_title {
            warn!("Called AE '{}' does not match our AE '{}'", called_ae, self.ae_title);
            self.send_association_reject(socket, 1, 1, 3).await?; // Called AE title not recognized
            return Ok(None);
        }

        // Evaluate presentation contexts
        let mut accepted_contexts = Vec::new();
        for mut pc in presentation_contexts {
            pc.result = self.evaluate_presentation_context(&pc);
            accepted_contexts.push(pc);
        }

        // Send A-ASSOCIATE-AC
        self.send_association_accept(socket, &calling_ae, &accepted_contexts).await?;

        Ok(Some(AssociationContext {
            calling_ae,
            called_ae,
            presentation_contexts: accepted_contexts,
            max_pdu_length: 16384, // Default PDU length
        }))
    }

    fn parse_association_request(&self, data: &[u8]) -> Result<(String, String, Vec<PresentationContext>)> {
        let mut offset = 0;
        
        // Check minimum data length
        if data.len() < 68 {
            return Err(anyhow::anyhow!("Association request too short: {} bytes", data.len()));
        }
        
        // Skip protocol version (2 bytes)
        offset += 2;
        
        // Skip reserved fields (2 bytes)
        offset += 2;
        
        // Read called AE title (16 bytes)
        if offset + 16 > data.len() {
            return Err(anyhow::anyhow!("Invalid association request: not enough data for called AE"));
        }
        let called_ae = String::from_utf8_lossy(&data[offset..offset + 16]).trim().to_string();
        offset += 16;
        
        // Read calling AE title (16 bytes)
        if offset + 16 > data.len() {
            return Err(anyhow::anyhow!("Invalid association request: not enough data for calling AE"));
        }
        let calling_ae = String::from_utf8_lossy(&data[offset..offset + 16]).trim().to_string();
        offset += 16;
        
        // Skip reserved fields (32 bytes)
        if offset + 32 > data.len() {
            return Err(anyhow::anyhow!("Invalid association request: not enough data for reserved fields"));
        }
        offset += 32;
        
        // Parse variable items
        let mut presentation_contexts = Vec::new();
        
        while offset < data.len() {
            // Check if we have enough data for item header (4 bytes minimum)
            if offset + 4 > data.len() {
                break;
            }
            
            let item_type = data[offset];
            let item_length = u16::from_be_bytes([data[offset + 2], data[offset + 3]]) as usize;
            
            // Check if we have enough data for the full item
            if offset + 4 + item_length > data.len() {
                warn!("Truncated item in association request, skipping");
                break;
            }
            
            match item_type {
                0x20 => {
                    // Presentation Context Item
                    let pc = self.parse_presentation_context(&data[offset + 4..offset + 4 + item_length])?;
                    presentation_contexts.push(pc);
                }
                _ => {
                    // Skip unknown items
                    debug!("Skipping unknown item type: 0x{:02x}", item_type);
                }
            }
            
            offset += 4 + item_length;
        }
        
        Ok((calling_ae, called_ae, presentation_contexts))
    }

    fn parse_presentation_context(&self, data: &[u8]) -> Result<PresentationContext> {
        let id = data[0];
        
        let mut offset = 4; // Skip reserved bytes
        let mut abstract_syntax = String::new();
        let mut transfer_syntaxes = Vec::new();
        
        while offset < data.len() {
            let item_type = data[offset];
            let item_length = u16::from_be_bytes([data[offset + 2], data[offset + 3]]) as usize;
            
            match item_type {
                0x30 => {
                    // Abstract Syntax Sub-item
                    abstract_syntax = String::from_utf8_lossy(&data[offset + 4..offset + 4 + item_length]).to_string();
                }
                0x40 => {
                    // Transfer Syntax Sub-item
                    let ts = String::from_utf8_lossy(&data[offset + 4..offset + 4 + item_length]).to_string();
                    transfer_syntaxes.push(ts);
                }
                _ => {
                    debug!("Unknown presentation context sub-item: 0x{:02x}", item_type);
                }
            }
            
            offset += 4 + item_length;
        }
        
        Ok(PresentationContext {
            id,
            abstract_syntax,
            transfer_syntaxes,
            result: 0, // Will be set during evaluation
        })
    }

    fn evaluate_presentation_context(&self, pc: &PresentationContext) -> u8 {
        // Check if we support the abstract syntax (SOP Class)
        if !self.sop_registry.is_supported(&pc.abstract_syntax) {
            warn!("Unsupported abstract syntax: {}", pc.abstract_syntax);
            return 3; // Abstract syntax not supported
        }

        // Check if we support any of the transfer syntaxes
        let supported_syntaxes = get_default_transfer_syntaxes();
        for ts in &pc.transfer_syntaxes {
            if supported_syntaxes.contains(&ts.as_str()) {
                debug!("Accepting presentation context {} with transfer syntax {}", pc.id, ts);
                return 0; // Acceptance
            }
        }

        warn!("No supported transfer syntax found for context {}", pc.id);
        4 // Transfer syntaxes not supported
    }

    async fn send_association_reject(&self, socket: &mut TcpStream, result: u8, source: u8, reason: u8) -> Result<()> {
        let pdu = vec![
            0x03, 0x00, // A-ASSOCIATE-RJ PDU type and reserved
            0x00, 0x04, // PDU length (4 bytes)
            0x00, result, source, reason // Reserved, result, source, reason
        ];
        
        socket.write_all(&pdu).await?;
        Ok(())
    }

    async fn send_association_accept(&self, socket: &mut TcpStream, calling_ae: &str, contexts: &[PresentationContext]) -> Result<()> {
        let mut pdu_data = Vec::new();
        
        // Protocol version
        pdu_data.extend_from_slice(&[0x00, 0x01]);
        
        // Reserved
        pdu_data.extend_from_slice(&[0x00, 0x00]);
        
        // Called AE title (16 bytes, padded with spaces)
        let mut called_ae_bytes = [0x20u8; 16]; // Spaces
        let ae_bytes = self.ae_title.as_bytes();
        let copy_len = ae_bytes.len().min(16);
        called_ae_bytes[..copy_len].copy_from_slice(&ae_bytes[..copy_len]);
        pdu_data.extend_from_slice(&called_ae_bytes);
        
        // Calling AE title (16 bytes, padded with spaces)
        let mut calling_ae_bytes = [0x20u8; 16]; // Spaces
        let calling_bytes = calling_ae.as_bytes();
        let copy_len = calling_bytes.len().min(16);
        calling_ae_bytes[..copy_len].copy_from_slice(&calling_bytes[..copy_len]);
        pdu_data.extend_from_slice(&calling_ae_bytes);
        
        // Reserved (32 bytes)
        pdu_data.extend_from_slice(&[0u8; 32]);
        
        // Add presentation context items
        for pc in contexts {
            if pc.result == 0 {
                // Only include accepted contexts
                let mut pc_data = Vec::new();
                pc_data.push(pc.id);
                pc_data.extend_from_slice(&[0x00, pc.result, 0x00]); // Reserved, result, reserved
                
                // Add transfer syntax sub-item (use first supported one)
                if let Some(ts) = pc.transfer_syntaxes.first() {
                    pc_data.push(0x40); // Transfer syntax sub-item type
                    pc_data.push(0x00); // Reserved
                    
                    let ts_bytes = ts.as_bytes();
                    pc_data.extend_from_slice(&(ts_bytes.len() as u16).to_be_bytes());
                    pc_data.extend_from_slice(ts_bytes);
                }
                
                // Add presentation context item header
                pdu_data.push(0x21); // Presentation context item (AC)
                pdu_data.push(0x00); // Reserved
                pdu_data.extend_from_slice(&(pc_data.len() as u16).to_be_bytes());
                pdu_data.extend_from_slice(&pc_data);
            }
        }
        
        // Create PDU header
        let mut pdu = vec![0x02, 0x00]; // A-ASSOCIATE-AC PDU type and reserved
        pdu.extend_from_slice(&(pdu_data.len() as u32).to_be_bytes()); // PDU length (4 bytes)
        pdu.extend_from_slice(&pdu_data);
        
        debug!("Sending A-ASSOCIATE-AC with {} bytes", pdu.len());
        socket.write_all(&pdu).await?;
        info!("Sent A-ASSOCIATE-AC response");
        Ok(())
    }

    async fn handle_c_store_requests(&self, socket: &mut TcpStream, ctx: &AssociationContext) -> Result<()> {
        info!("Waiting for C-STORE requests...");
        loop {
            // Read PDU header
            let mut pdu_header = [0u8; 6];
            match socket.read_exact(&mut pdu_header).await {
                Ok(_) => {},
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    info!("Connection closed by peer");
                    break;
                }
                Err(e) => {
                    error!("Error reading PDU header: {}", e);
                    return Err(e.into());
                }
            }

            let pdu_type = pdu_header[0];
            // PDU length is in bytes 2-5 (4 bytes, big endian)
            let pdu_length = u32::from_be_bytes([pdu_header[2], pdu_header[3], pdu_header[4], pdu_header[5]]) as usize;
            info!("Received PDU type: 0x{:02x}, length: {}", pdu_type, pdu_length);

            match pdu_type {
                0x04 => {
                    // P-DATA-TF PDU
                    let mut pdu_data = vec![0u8; pdu_length];
                    socket.read_exact(&mut pdu_data).await?;
                    
                    if let Err(e) = self.handle_p_data(&pdu_data, ctx).await {
                        error!("Error handling P-DATA: {}", e);
                        // Send C-STORE response with failure status
                        // Implementation would depend on parsing the original message
                    }
                }
                0x05 => {
                    // A-RELEASE-RQ PDU
                    info!("Received A-RELEASE-RQ");
                    self.send_release_response(socket).await?;
                    break;
                }
                0x07 => {
                    // A-ABORT PDU
                    info!("Received A-ABORT");
                    break;
                }
                _ => {
                    warn!("Unknown PDU type: 0x{:02x}", pdu_type);
                    break;
                }
            }
        }

        Ok(())
    }

    async fn handle_p_data(&self, data: &[u8], _ctx: &AssociationContext) -> Result<()> {
        // This is a simplified implementation
        // In a real implementation, you would:
        // 1. Parse the DIMSE message (C-STORE-RQ)
        // 2. Extract the DICOM dataset
        // 3. Save it to disk
        // 4. Send C-STORE-RSP

        debug!("Received P-DATA with {} bytes", data.len());

        // For now, just save raw data with a timestamp
        let filename = format!("received_{}.dcm", Utc::now().format("%Y%m%d_%H%M%S_%f"));
        let file_path = self.output_dir.join(filename);

        // In a real implementation, you would parse the DICOM data properly
        fs::write(&file_path, data).await?;
        
        info!("Saved received data to {}", file_path.display());

        Ok(())
    }

    async fn send_release_response(&self, socket: &mut TcpStream) -> Result<()> {
        let pdu = vec![
            0x06, 0x00, // A-RELEASE-RP PDU type and reserved
            0x00, 0x04, // PDU length (4 bytes)
            0x00, 0x00, 0x00, 0x00 // Reserved
        ];
        
        socket.write_all(&pdu).await?;
        Ok(())
    }
}
