/// Comprehensive DICOM SOP Class definitions
/// 
/// This module provides a wide range of SOP Class UIDs covering most common
/// DICOM use cases including imaging, structured reporting, and specialized modalities.

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct SopClassInfo {
    pub uid: &'static str,
    pub name: &'static str,
    pub category: SopClassCategory,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SopClassCategory {
    // Core imaging modalities
    ComputedRadiography,
    ComputedTomography,
    MagneticResonance,
    Ultrasound,
    NuclearMedicine,
    DigitalRadiography,
    DigitalMammography,
    
    // Specialized imaging
    PetCt,
    OpticalCoherenceTomography,
    Endoscopy,
    Microscopy,
    
    // Non-image objects
    StructuredReporting,
    Presentation,
    Waveform,
    RawData,
    
    // Secondary and derived
    SecondaryCapture,
    KeyObjectSelection,
    
    // Enhanced and multi-frame
    Enhanced,
    MultiFrame,
    
    // Specialized applications
    Radiotherapy,
    Ophthalmology,
    Dermatology,
    Dental,
    
    // Legacy and other
    Legacy,
    Other,
}

impl SopClassInfo {
    pub const fn new(uid: &'static str, name: &'static str, category: SopClassCategory) -> Self {
        Self { uid, name, category }
    }
}

/// Comprehensive SOP Class registry
#[derive(Debug)]
pub struct SopClassRegistry {
    classes: HashMap<&'static str, SopClassInfo>,
}

impl SopClassRegistry {
    pub fn new() -> Self {
        let mut classes = HashMap::new();
        
        // Register all SOP classes
        for sop_class in ALL_SOP_CLASSES {
            classes.insert(sop_class.uid, sop_class.clone());
        }
        
        Self { classes }
    }
    
    pub fn get(&self, uid: &str) -> Option<&SopClassInfo> {
        self.classes.get(uid)
    }
    
    pub fn get_all_uids(&self) -> Vec<&'static str> {
        ALL_SOP_CLASSES.iter().map(|sc| sc.uid).collect()
    }
    
    pub fn get_by_category(&self, category: SopClassCategory) -> Vec<&SopClassInfo> {
        ALL_SOP_CLASSES.iter()
            .filter(|sc| sc.category == category)
            .collect()
    }
    
    pub fn is_supported(&self, uid: &str) -> bool {
        self.classes.contains_key(uid)
    }
    
    pub fn get_name(&self, uid: &str) -> Option<&'static str> {
        self.get(uid).map(|sc| sc.name)
    }
}

impl Default for SopClassRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// Comprehensive SOP Class definitions
const ALL_SOP_CLASSES: &[SopClassInfo] = &[
    // =============================================================================
    // IMAGE STORAGE SOP CLASSES
    // =============================================================================
    
    // Computed Radiography
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.1",
        "Computed Radiography Image Storage",
        SopClassCategory::ComputedRadiography,
    ),
    
    // Digital X-Ray
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.1.1",
        "Digital X-Ray Image Storage - For Presentation",
        SopClassCategory::DigitalRadiography,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.1.1.1",
        "Digital X-Ray Image Storage - For Processing",
        SopClassCategory::DigitalRadiography,
    ),
    
    // Digital Mammography
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.1.2",
        "Digital Mammography X-Ray Image Storage - For Presentation",
        SopClassCategory::DigitalMammography,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.1.2.1",
        "Digital Mammography X-Ray Image Storage - For Processing",
        SopClassCategory::DigitalMammography,
    ),
    
    // Digital Intra-Oral X-Ray
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.1.3",
        "Digital Intra-Oral X-Ray Image Storage - For Presentation",
        SopClassCategory::Dental,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.1.3.1",
        "Digital Intra-Oral X-Ray Image Storage - For Processing",
        SopClassCategory::Dental,
    ),
    
    // CT Image Storage
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.2",
        "CT Image Storage",
        SopClassCategory::ComputedTomography,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.2.1",
        "Enhanced CT Image Storage",
        SopClassCategory::Enhanced,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.2.2",
        "Legacy Converted Enhanced CT Image Storage",
        SopClassCategory::Legacy,
    ),
    
    // Ultrasound
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.3.1",
        "Ultrasound Multi-frame Image Storage",
        SopClassCategory::Ultrasound,
    ),
    
    // MR Image Storage
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.4",
        "MR Image Storage",
        SopClassCategory::MagneticResonance,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.4.1",
        "Enhanced MR Image Storage",
        SopClassCategory::Enhanced,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.4.2",
        "MR Spectroscopy Storage",
        SopClassCategory::MagneticResonance,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.4.3",
        "Enhanced MR Color Image Storage",
        SopClassCategory::Enhanced,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.4.4",
        "Legacy Converted Enhanced MR Image Storage",
        SopClassCategory::Legacy,
    ),
    
    // Nuclear Medicine
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.5",
        "Nuclear Medicine Image Storage (Retired)",
        SopClassCategory::Legacy,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.20",
        "Nuclear Medicine Image Storage",
        SopClassCategory::NuclearMedicine,
    ),
    
    // Ultrasound Image Storage
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.6.1",
        "Ultrasound Image Storage",
        SopClassCategory::Ultrasound,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.6.2",
        "Enhanced US Volume Storage",
        SopClassCategory::Enhanced,
    ),
    
    // Secondary Capture
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.7",
        "Secondary Capture Image Storage",
        SopClassCategory::SecondaryCapture,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.7.1",
        "Multi-frame Single Bit Secondary Capture Image Storage",
        SopClassCategory::SecondaryCapture,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.7.2",
        "Multi-frame Grayscale Byte Secondary Capture Image Storage",
        SopClassCategory::SecondaryCapture,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.7.3",
        "Multi-frame Grayscale Word Secondary Capture Image Storage",
        SopClassCategory::SecondaryCapture,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.7.4",
        "Multi-frame True Color Secondary Capture Image Storage",
        SopClassCategory::SecondaryCapture,
    ),
    
    // X-Ray Angiographic
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.12.1",
        "X-Ray Angiographic Image Storage",
        SopClassCategory::DigitalRadiography,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.12.1.1",
        "Enhanced XA Image Storage",
        SopClassCategory::Enhanced,
    ),
    
    // X-Ray Radiofluoroscopic
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.12.2",
        "X-Ray Radiofluoroscopic Image Storage",
        SopClassCategory::DigitalRadiography,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.12.2.1",
        "Enhanced XRF Image Storage",
        SopClassCategory::Enhanced,
    ),
    
    // PET
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.128",
        "Positron Emission Tomography Image Storage",
        SopClassCategory::NuclearMedicine,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.130",
        "Enhanced PET Image Storage",
        SopClassCategory::Enhanced,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.131",
        "Legacy Converted Enhanced PET Image Storage",
        SopClassCategory::Legacy,
    ),
    
    // RT (Radiotherapy)
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.481.1",
        "RT Image Storage",
        SopClassCategory::Radiotherapy,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.481.2",
        "RT Dose Storage",
        SopClassCategory::Radiotherapy,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.481.3",
        "RT Structure Set Storage",
        SopClassCategory::Radiotherapy,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.481.4",
        "RT Beams Treatment Record Storage",
        SopClassCategory::Radiotherapy,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.481.5",
        "RT Plan Storage",
        SopClassCategory::Radiotherapy,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.481.6",
        "RT Brachy Treatment Record Storage",
        SopClassCategory::Radiotherapy,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.481.7",
        "RT Treatment Summary Record Storage",
        SopClassCategory::Radiotherapy,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.481.8",
        "RT Ion Plan Storage",
        SopClassCategory::Radiotherapy,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.481.9",
        "RT Ion Beams Treatment Record Storage",
        SopClassCategory::Radiotherapy,
    ),
    
    // =============================================================================
    // WAVEFORM STORAGE SOP CLASSES
    // =============================================================================
    
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.9.1.1",
        "12-lead ECG Waveform Storage",
        SopClassCategory::Waveform,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.9.1.2",
        "General ECG Waveform Storage",
        SopClassCategory::Waveform,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.9.1.3",
        "Ambulatory ECG Waveform Storage",
        SopClassCategory::Waveform,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.9.2.1",
        "Hemodynamic Waveform Storage",
        SopClassCategory::Waveform,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.9.3.1",
        "Cardiac Electrophysiology Waveform Storage",
        SopClassCategory::Waveform,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.9.4.1",
        "Basic Voice Audio Waveform Storage",
        SopClassCategory::Waveform,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.9.4.2",
        "General Audio Waveform Storage",
        SopClassCategory::Waveform,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.9.5.1",
        "Arterial Pulse Waveform Storage",
        SopClassCategory::Waveform,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.9.6.1",
        "Respiratory Waveform Storage",
        SopClassCategory::Waveform,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.9.6.2",
        "Multi-channel Respiratory Waveform Storage",
        SopClassCategory::Waveform,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.9.7.1",
        "Routine Scalp Electroencephalogram Waveform Storage",
        SopClassCategory::Waveform,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.9.7.2",
        "Electromyogram Waveform Storage",
        SopClassCategory::Waveform,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.9.7.3",
        "Electrooculogram Waveform Storage",
        SopClassCategory::Waveform,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.9.7.4",
        "Sleep Electroencephalogram Waveform Storage",
        SopClassCategory::Waveform,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.9.8.1",
        "Body Position Waveform Storage",
        SopClassCategory::Waveform,
    ),
    
    // =============================================================================
    // STRUCTURED REPORTING SOP CLASSES
    // =============================================================================
    
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.88.11",
        "Basic Text SR Storage",
        SopClassCategory::StructuredReporting,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.88.22",
        "Enhanced SR Storage",
        SopClassCategory::StructuredReporting,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.88.33",
        "Comprehensive SR Storage",
        SopClassCategory::StructuredReporting,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.88.40",
        "Procedure Log Storage",
        SopClassCategory::StructuredReporting,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.88.50",
        "Mammography CAD SR Storage",
        SopClassCategory::StructuredReporting,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.88.59",
        "Key Object Selection Document Storage",
        SopClassCategory::KeyObjectSelection,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.88.65",
        "Chest CAD SR Storage",
        SopClassCategory::StructuredReporting,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.88.67",
        "X-Ray Radiation Dose SR Storage",
        SopClassCategory::StructuredReporting,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.88.68",
        "Radiopharmaceutical Radiation Dose SR Storage",
        SopClassCategory::StructuredReporting,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.88.69",
        "Colon CAD SR Storage",
        SopClassCategory::StructuredReporting,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.88.70",
        "Implantation Plan SR Storage",
        SopClassCategory::StructuredReporting,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.88.71",
        "Acquisition Context SR Storage",
        SopClassCategory::StructuredReporting,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.88.72",
        "Simplified Adult Echo SR Storage",
        SopClassCategory::StructuredReporting,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.88.73",
        "Patient Radiation Dose SR Storage",
        SopClassCategory::StructuredReporting,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.88.74",
        "Planned Imaging Agent Administration SR Storage",
        SopClassCategory::StructuredReporting,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.88.75",
        "Performed Imaging Agent Administration SR Storage",
        SopClassCategory::StructuredReporting,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.88.76",
        "Enhanced X-Ray Radiation Dose SR Storage",
        SopClassCategory::StructuredReporting,
    ),
    
    // =============================================================================
    // RAW DATA STORAGE
    // =============================================================================
    
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.66",
        "Raw Data Storage",
        SopClassCategory::RawData,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.66.1",
        "Spatial Registration Storage",
        SopClassCategory::Other,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.66.2",
        "Spatial Fiducials Storage",
        SopClassCategory::Other,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.66.3",
        "Deformable Spatial Registration Storage",
        SopClassCategory::Other,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.66.4",
        "Segmentation Storage",
        SopClassCategory::Other,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.66.5",
        "Surface Segmentation Storage",
        SopClassCategory::Other,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.66.6",
        "Tractography Results Storage",
        SopClassCategory::Other,
    ),
    
    // =============================================================================
    // PRESENTATION STATE STORAGE
    // =============================================================================
    
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.11.1",
        "Grayscale Softcopy Presentation State Storage",
        SopClassCategory::Presentation,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.11.2",
        "Color Softcopy Presentation State Storage",
        SopClassCategory::Presentation,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.11.3",
        "Pseudo-Color Softcopy Presentation State Storage",
        SopClassCategory::Presentation,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.11.4",
        "Blending Softcopy Presentation State Storage",
        SopClassCategory::Presentation,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.11.5",
        "XA/XRF Grayscale Softcopy Presentation State Storage",
        SopClassCategory::Presentation,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.11.6",
        "Grayscale Planar MPR Volumetric Presentation State Storage",
        SopClassCategory::Presentation,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.11.7",
        "Compositing Planar MPR Volumetric Presentation State Storage",
        SopClassCategory::Presentation,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.11.8",
        "Advanced Blending Presentation State Storage",
        SopClassCategory::Presentation,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.11.9",
        "Volume Rendering Volumetric Presentation State Storage",
        SopClassCategory::Presentation,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.11.10",
        "Segmented Volume Rendering Volumetric Presentation State Storage",
        SopClassCategory::Presentation,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.11.11",
        "Multiple Volume Rendering Volumetric Presentation State Storage",
        SopClassCategory::Presentation,
    ),
    
    // =============================================================================
    // OPHTHALMIC IMAGING
    // =============================================================================
    
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.77.1.1",
        "Video Endoscopic Image Storage",
        SopClassCategory::Endoscopy,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.77.1.2",
        "Video Microscopic Image Storage",
        SopClassCategory::Microscopy,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.77.1.3",
        "Video Photographic Image Storage",
        SopClassCategory::Other,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.77.1.4",
        "Ophthalmic Photography 8 Bit Image Storage",
        SopClassCategory::Ophthalmology,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.77.1.4.1",
        "Ophthalmic Photography 16 Bit Image Storage",
        SopClassCategory::Ophthalmology,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.77.1.5.1",
        "Stereometric Relationship Storage",
        SopClassCategory::Ophthalmology,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.77.1.5.2",
        "Ophthalmic Tomography Image Storage",
        SopClassCategory::OpticalCoherenceTomography,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.77.1.5.3",
        "Wide Field Ophthalmic Photography Stereographic Projection Image Storage",
        SopClassCategory::Ophthalmology,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.77.1.5.4",
        "Wide Field Ophthalmic Photography 3D Coordinates Image Storage",
        SopClassCategory::Ophthalmology,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.77.1.5.5",
        "Ophthalmic Optical Coherence Tomography En Face Image Storage",
        SopClassCategory::OpticalCoherenceTomography,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.77.1.5.6",
        "Ophthalmic Optical Coherence Tomography B-scan Volume Analysis Storage",
        SopClassCategory::OpticalCoherenceTomography,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.77.1.5.7",
        "VL Whole Slide Microscopy Image Storage",
        SopClassCategory::Microscopy,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.77.1.5.8",
        "Dermoscopic Photography Image Storage",
        SopClassCategory::Dermatology,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.77.1.6",
        "Ophthalmic Visual Field Static Perimetry Measurements Storage",
        SopClassCategory::Ophthalmology,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.77.1.7",
        "Ophthalmic Thickness Map Storage",
        SopClassCategory::Ophthalmology,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.77.1.8",
        "Corneal Topography Map Storage",
        SopClassCategory::Ophthalmology,
    ),
    
    // =============================================================================
    // ADDITIONAL SPECIALIZED SOP CLASSES
    // =============================================================================
    
    // Multi-frame and Enhanced
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.3",
        "Ultrasound Multi-frame Image Storage (Retired)",
        SopClassCategory::Legacy,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.8",
        "Standalone Overlay Storage (Retired)",
        SopClassCategory::Legacy,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.10",
        "Standalone Curve Storage (Retired)",
        SopClassCategory::Legacy,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.1.1.129",
        "Standalone PET Curve Storage (Retired)",
        SopClassCategory::Legacy,
    ),
    
    // Basic Directory and Media Storage
    SopClassInfo::new(
        "1.2.840.10008.1.3.10",
        "Media Storage Directory Storage",
        SopClassCategory::Other,
    ),
    
    // Hanging Protocols
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.38.1",
        "Hanging Protocol Storage",
        SopClassCategory::Other,
    ),
    
    // Color Palette
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.39.1",
        "Color Palette Storage",
        SopClassCategory::Other,
    ),
    
    // Generic Implant Template
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.43.1",
        "Generic Implant Template Storage",
        SopClassCategory::Other,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.44.1",
        "Implant Assembly Template Storage",
        SopClassCategory::Other,
    ),
    SopClassInfo::new(
        "1.2.840.10008.5.1.4.45.1",
        "Implant Template Group Storage",
        SopClassCategory::Other,
    ),
];

/// Helper function to get a default transfer syntax list for any SOP class
pub fn get_default_transfer_syntaxes() -> Vec<&'static str> {
    super::transfer_syntaxes::get_basic_transfer_syntaxes()
}

/// Helper function to get lossless transfer syntax list for medical imaging
pub fn get_lossless_transfer_syntaxes() -> Vec<&'static str> {
    super::transfer_syntaxes::get_lossless_transfer_syntaxes()
}

/// Helper function to get comprehensive transfer syntax list including compressed formats
pub fn get_comprehensive_transfer_syntaxes() -> Vec<&'static str> {
    super::transfer_syntaxes::get_comprehensive_transfer_syntaxes()
}

/// Get appropriate transfer syntaxes based on SOP class category
pub fn get_transfer_syntaxes_for_category(category: &SopClassCategory) -> Vec<&'static str> {
    match category {
        // Core imaging modalities benefit from comprehensive support including compression
        SopClassCategory::ComputedTomography |
        SopClassCategory::MagneticResonance |
        SopClassCategory::DigitalRadiography |
        SopClassCategory::DigitalMammography => {
            super::transfer_syntaxes::get_compressed_transfer_syntaxes()
        }
        
        // Enhanced formats should support all modern transfer syntaxes
        SopClassCategory::Enhanced => {
            super::transfer_syntaxes::get_comprehensive_transfer_syntaxes()
        }
        
        // Waveforms and raw data typically use lossless compression
        SopClassCategory::Waveform |
        SopClassCategory::RawData => {
            super::transfer_syntaxes::get_lossless_transfer_syntaxes()
        }
        
        // Video applications need video transfer syntaxes
        SopClassCategory::Endoscopy |
        SopClassCategory::Microscopy => {
            let mut syntaxes = super::transfer_syntaxes::get_compressed_transfer_syntaxes();
            syntaxes.extend(super::transfer_syntaxes::get_video_transfer_syntaxes());
            syntaxes
        }
        
        // Legacy formats stick to basic transfer syntaxes for compatibility
        SopClassCategory::Legacy => {
            super::transfer_syntaxes::get_basic_transfer_syntaxes()
        }
        
        // All others get lossless by default (conservative approach)
        _ => {
            super::transfer_syntaxes::get_lossless_transfer_syntaxes()
        }
    }
}

/// Helper function to get extended transfer syntax list for enhanced SOP classes (deprecated - use get_comprehensive_transfer_syntaxes)
#[deprecated(note = "Use get_comprehensive_transfer_syntaxes() instead")]
pub fn get_extended_transfer_syntaxes() -> Vec<&'static str> {
    get_comprehensive_transfer_syntaxes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sop_class_registry() {
        let registry = SopClassRegistry::new();
        
        // Test basic lookups
        assert!(registry.is_supported("1.2.840.10008.5.1.4.1.1.2")); // CT
        assert!(registry.is_supported("1.2.840.10008.5.1.4.1.1.4")); // MR
        assert!(!registry.is_supported("invalid.uid"));
        
        // Test name lookup
        assert_eq!(
            registry.get_name("1.2.840.10008.5.1.4.1.1.2"),
            Some("CT Image Storage")
        );
        
        // Test category filtering
        let ct_classes = registry.get_by_category(SopClassCategory::ComputedTomography);
        assert!(!ct_classes.is_empty());
        
        // Test that we have a comprehensive list
        let all_uids = registry.get_all_uids();
        assert!(all_uids.len() > 100); // Should have many SOP classes
    }
    
    #[test]
    fn test_transfer_syntaxes() {
        let basic = get_default_transfer_syntaxes();
        let extended = get_extended_transfer_syntaxes();
        
        assert!(!basic.is_empty());
        assert!(extended.len() > basic.len());
        
        // Basic should be subset of extended
        for ts in &basic {
            assert!(extended.contains(ts));
        }
    }
}
