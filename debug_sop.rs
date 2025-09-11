use dicom::object::open_file;
use dicom_core::header::Tag;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let obj = open_file("test-dicom/004f0647-833b-451a-adbd-b8336c8260ea.dcm")?;
    
    let sop_class_element = obj.element(Tag(0x0008, 0x0016))?;
    println!("Raw SOP Class element: {:?}", sop_class_element);
    
    let sop_class_uid = sop_class_element.string()?;
    println!("SOP Class UID string: '{}'", sop_class_uid);
    println!("SOP Class UID trimmed: '{}'", sop_class_uid.trim());
    println!("SOP Class UID length: {}", sop_class_uid.len());
    println!("SOP Class UID bytes: {:?}", sop_class_uid.as_bytes());
    
    Ok(())
}
