use rust_dicom::common::sop_classes::SopClassRegistry;
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
    let registry = SopClassRegistry::new();
    let all_uids = registry.get_all_uids();
    
    println!("🚀 Comprehensive SOP Class Support");
    println!("================================");
    println!("Total SOP Classes Registered: {}", all_uids.len());
    println!();
    
    // Show a few examples from each category
    println!("📋 Sample SOP Classes by Category:");
    println!();
    
    // Show first few from our registry
    let mut count = 0;
    for uid in all_uids.iter().take(20) {
        if let Some(info) = registry.get(uid) {
            println!("• {} ({:?})", info.name, info.category);
            count += 1;
        }
        // Flush output and check for broken pipe
        io::stdout().flush()?;
    }
    
    if all_uids.len() > 20 {
        println!("... and {} more SOP classes", all_uids.len() - 20);
    }
    
    println!();
    println!("🎯 This means our DICOM sender now supports:");
    println!("• All major imaging modalities (CT, MR, US, XR, etc.)");
    println!("• Enhanced and multi-frame formats");
    println!("• Specialized modalities (PET, Nuclear Medicine, RT)");
    println!("• Structured reporting and presentation states");
    println!("• Waveforms and raw data storage");
    println!("• Legacy and specialized SOP classes");
    
    io::stdout().flush()?;
    Ok(())
}
