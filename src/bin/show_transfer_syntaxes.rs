use rust_dicom::common::transfer_syntaxes::{TransferSyntaxRegistry, TransferSyntaxCategory, CompressionType};
use rust_dicom::common::sop_classes::{SopClassRegistry, get_transfer_syntaxes_for_category, SopClassCategory};
use std::io::{self, Write};

fn main() {
    let result = run();
    if let Err(e) = result {
        if e.kind() == io::ErrorKind::BrokenPipe {
            // Ignore broken pipe errors (e.g., when piped to `head`)
            std::process::exit(0);
        } else {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn run() -> io::Result<()> {
    let ts_registry = TransferSyntaxRegistry::new();
    
    println!("🚀 Comprehensive Transfer Syntax Support");
    println!("=====================================");
    println!("Total Transfer Syntaxes: {}", ts_registry.get_all_uids().len());
    println!();
    
    // Show transfer syntaxes by category
    show_by_category(&ts_registry, TransferSyntaxCategory::Uncompressed, "📋 Uncompressed Transfer Syntaxes")?;
    show_by_category(&ts_registry, TransferSyntaxCategory::LosslessCompressed, "🔒 Lossless Compressed Transfer Syntaxes")?;
    
    // Try to flush and catch potential broken pipe early
    if let Err(e) = io::stdout().flush() {
        if e.kind() == io::ErrorKind::BrokenPipe {
            return Ok(());
        }
        return Err(e);
    }
    
    show_by_category(&ts_registry, TransferSyntaxCategory::LossyCompressed, "📉 Lossy Compressed Transfer Syntaxes")?;
    show_by_category(&ts_registry, TransferSyntaxCategory::Video, "🎬 Video Transfer Syntaxes")?;
    show_by_category(&ts_registry, TransferSyntaxCategory::Legacy, "📜 Legacy Transfer Syntaxes")?;
    
    println!();
    println!("✨ Key Benefits:");
    println!("• Automatic transfer syntax selection based on SOP class type");
    println!("• Support for modern compression (JPEG 2000, JPEG-LS, etc.)");
    println!("• Video codec support (H.264, H.265, MPEG-2)");
    println!("• Lossless preservation for critical imaging");
    println!("• Backward compatibility with legacy systems");
    println!("• Comprehensive coverage of all DICOM transfer syntaxes");
    
    io::stdout().flush()?;
    Ok(())
}

fn show_by_category(registry: &TransferSyntaxRegistry, category: TransferSyntaxCategory, title: &str) -> io::Result<()> {
    println!("{}", title);
    println!("{}", "=".repeat(title.len()));
    
    let syntaxes = registry.get_by_category(category);
    for ts in syntaxes.iter().take(8) { // Show first 8 to keep output manageable
        if matches!(ts.compression, CompressionType::None) {
            println!("• {}", ts.name);
        } else {
            println!("• {} [{:?}]", ts.name, ts.compression);
        }
    }
    
    if syntaxes.len() > 8 {
        println!("... and {} more", syntaxes.len() - 8);
    }
    
    println!();
    
    // Only flush once per category
    io::stdout().flush()?;
    Ok(())
}

fn show_smart_selection(sop_registry: &SopClassRegistry, category: SopClassCategory, name: &str) -> io::Result<()> {
    let transfer_syntaxes = get_transfer_syntaxes_for_category(&category);
    
    println!("📌 {} ({} transfer syntaxes):", name, transfer_syntaxes.len());
    
    let ts_registry = TransferSyntaxRegistry::new();
    let mut shown = 0;
    for ts_uid in transfer_syntaxes.iter().take(5) {
        if let Some(ts_info) = ts_registry.get(ts_uid) {
            println!("  • {}", ts_info.name);
            shown += 1;
            io::stdout().flush()?;
        }
    }
    
    if transfer_syntaxes.len() > shown {
        println!("  ... and {} more", transfer_syntaxes.len() - shown);
    }
    
    println!();
    io::stdout().flush()?;
    Ok(())
}
