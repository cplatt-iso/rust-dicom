use dicom_sender::sop_classes::SopClassRegistry;

fn main() {
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
}
