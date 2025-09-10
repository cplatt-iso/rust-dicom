use dicom_sender::transfer_syntaxes::{TransferSyntaxRegistry, TransferSyntaxCategory, CompressionType};
use dicom_sender::sop_classes::{SopClassRegistry, get_transfer_syntaxes_for_category, SopClassCategory};

fn main() {
    let ts_registry = TransferSyntaxRegistry::new();
    let sop_registry = SopClassRegistry::new();
    
    println!("🚀 Comprehensive Transfer Syntax Support");
    println!("=====================================");
    println!("Total Transfer Syntaxes: {}", ts_registry.get_all_uids().len());
    println!();
    
    // Show transfer syntaxes by category
    show_by_category(&ts_registry, TransferSyntaxCategory::Uncompressed, "📋 Uncompressed Transfer Syntaxes");
    show_by_category(&ts_registry, TransferSyntaxCategory::LosslessCompressed, "🔒 Lossless Compressed Transfer Syntaxes");
    show_by_category(&ts_registry, TransferSyntaxCategory::LossyCompressed, "📉 Lossy Compressed Transfer Syntaxes");
    show_by_category(&ts_registry, TransferSyntaxCategory::Video, "🎬 Video Transfer Syntaxes");
    show_by_category(&ts_registry, TransferSyntaxCategory::Legacy, "📜 Legacy Transfer Syntaxes");
    
    println!();
    println!("🎯 Smart Transfer Syntax Selection Examples:");
    println!("==========================================");
    
    // Show examples of smart selection for different SOP class categories
    show_smart_selection(&sop_registry, SopClassCategory::ComputedTomography, "CT Imaging");
    show_smart_selection(&sop_registry, SopClassCategory::Enhanced, "Enhanced Formats");
    show_smart_selection(&sop_registry, SopClassCategory::Waveform, "Waveforms");
    show_smart_selection(&sop_registry, SopClassCategory::Endoscopy, "Endoscopy/Video");
    show_smart_selection(&sop_registry, SopClassCategory::Legacy, "Legacy Formats");
    
    println!();
    println!("✨ Key Benefits:");
    println!("• Automatic transfer syntax selection based on SOP class type");
    println!("• Support for modern compression (JPEG 2000, JPEG-LS, etc.)");
    println!("• Video codec support (H.264, H.265, MPEG-2)");
    println!("• Lossless preservation for critical imaging");
    println!("• Backward compatibility with legacy systems");
    println!("• Comprehensive coverage of all DICOM transfer syntaxes");
}

fn show_by_category(registry: &TransferSyntaxRegistry, category: TransferSyntaxCategory, title: &str) {
    println!("{}", title);
    println!("{}", "=".repeat(title.len()));
    
    let syntaxes = registry.get_by_category(category);
    for ts in syntaxes.iter().take(8) { // Show first 8 to keep output manageable
        let compression_info = match ts.compression {
            CompressionType::None => "",
            _ => &format!(" [{}]", format!("{:?}", ts.compression)),
        };
        
        println!("• {}{}", ts.name, compression_info);
    }
    
    if syntaxes.len() > 8 {
        println!("... and {} more", syntaxes.len() - 8);
    }
    
    println!();
}

fn show_smart_selection(sop_registry: &SopClassRegistry, category: SopClassCategory, name: &str) {
    let transfer_syntaxes = get_transfer_syntaxes_for_category(&category);
    
    println!("📌 {} ({} transfer syntaxes):", name, transfer_syntaxes.len());
    
    let ts_registry = TransferSyntaxRegistry::new();
    let mut shown = 0;
    for ts_uid in transfer_syntaxes.iter().take(5) {
        if let Some(ts_info) = ts_registry.get(ts_uid) {
            println!("  • {}", ts_info.name);
            shown += 1;
        }
    }
    
    if transfer_syntaxes.len() > shown {
        println!("  ... and {} more", transfer_syntaxes.len() - shown);
    }
    
    println!();
}
