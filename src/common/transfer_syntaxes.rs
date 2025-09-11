/// Comprehensive DICOM Transfer Syntax support
/// 
/// This module provides comprehensive transfer syntax definitions and utilities
/// for negotiating and handling various DICOM transfer syntaxes including
/// uncompressed, lossless compressed, and lossy compressed formats.

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum TransferSyntaxCategory {
    /// Uncompressed transfer syntaxes
    Uncompressed,
    /// Lossless compression
    LosslessCompressed,
    /// Lossy compression
    LossyCompressed,
    /// Legacy or specialized
    Legacy,
    /// Video compression
    Video,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CompressionType {
    None,
    JPEG,
    JPEGLossless,
    JPEGLS,
    JPEG2000,
    RLE,
    MPEG2,
    MPEG4,
    H264,
    H265,
}

#[derive(Debug, Clone)]
pub struct TransferSyntaxInfo {
    pub uid: &'static str,
    pub name: &'static str,
    pub category: TransferSyntaxCategory,
    pub compression: CompressionType,
    pub is_little_endian: bool,
    pub is_explicit_vr: bool,
    pub supports_encapsulation: bool,
}

impl TransferSyntaxInfo {
    pub const fn new(
        uid: &'static str,
        name: &'static str,
        category: TransferSyntaxCategory,
        compression: CompressionType,
        is_little_endian: bool,
        is_explicit_vr: bool,
        supports_encapsulation: bool,
    ) -> Self {
        Self {
            uid,
            name,
            category,
            compression,
            is_little_endian,
            is_explicit_vr,
            supports_encapsulation,
        }
    }

    pub fn is_compressed(&self) -> bool {
        self.compression != CompressionType::None
    }

    pub fn is_lossless(&self) -> bool {
        matches!(
            self.category,
            TransferSyntaxCategory::Uncompressed | TransferSyntaxCategory::LosslessCompressed
        )
    }
}

/// Comprehensive Transfer Syntax registry
#[derive(Debug)]
pub struct TransferSyntaxRegistry {
    syntaxes: HashMap<&'static str, TransferSyntaxInfo>,
}

impl TransferSyntaxRegistry {
    pub fn new() -> Self {
        let mut syntaxes = HashMap::new();
        
        // Register all transfer syntaxes
        for ts in ALL_TRANSFER_SYNTAXES {
            syntaxes.insert(ts.uid, ts.clone());
        }
        
        Self { syntaxes }
    }
    
    pub fn get(&self, uid: &str) -> Option<&TransferSyntaxInfo> {
        self.syntaxes.get(uid)
    }
    
    pub fn get_all_uids(&self) -> Vec<&'static str> {
        ALL_TRANSFER_SYNTAXES.iter().map(|ts| ts.uid).collect()
    }
    
    pub fn get_by_category(&self, category: TransferSyntaxCategory) -> Vec<&TransferSyntaxInfo> {
        ALL_TRANSFER_SYNTAXES.iter()
            .filter(|ts| ts.category == category)
            .collect()
    }
    
    pub fn get_uncompressed(&self) -> Vec<&'static str> {
        self.get_by_category(TransferSyntaxCategory::Uncompressed)
            .iter()
            .map(|ts| ts.uid)
            .collect()
    }
    
    pub fn get_lossless_compressed(&self) -> Vec<&'static str> {
        self.get_by_category(TransferSyntaxCategory::LosslessCompressed)
            .iter()
            .map(|ts| ts.uid)
            .collect()
    }
    
    pub fn get_lossy_compressed(&self) -> Vec<&'static str> {
        self.get_by_category(TransferSyntaxCategory::LossyCompressed)
            .iter()
            .map(|ts| ts.uid)
            .collect()
    }
    
    pub fn is_supported(&self, uid: &str) -> bool {
        self.syntaxes.contains_key(uid)
    }
    
    pub fn get_name(&self, uid: &str) -> Option<&'static str> {
        self.get(uid).map(|ts| ts.name)
    }
    
    pub fn is_compressed(&self, uid: &str) -> bool {
        self.get(uid).map_or(false, |ts| ts.is_compressed())
    }
    
    pub fn requires_encapsulation(&self, uid: &str) -> bool {
        self.get(uid).map_or(false, |ts| ts.supports_encapsulation)
    }
}

impl Default for TransferSyntaxRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// Comprehensive Transfer Syntax definitions
const ALL_TRANSFER_SYNTAXES: &[TransferSyntaxInfo] = &[
    // =============================================================================
    // UNCOMPRESSED TRANSFER SYNTAXES
    // =============================================================================
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2",
        "Implicit VR Little Endian",
        TransferSyntaxCategory::Uncompressed,
        CompressionType::None,
        true,  // little endian
        false, // implicit VR
        false, // no encapsulation
    ),
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.1",
        "Explicit VR Little Endian",
        TransferSyntaxCategory::Uncompressed,
        CompressionType::None,
        true,  // little endian
        true,  // explicit VR
        false, // no encapsulation
    ),
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.2",
        "Explicit VR Big Endian (Retired)",
        TransferSyntaxCategory::Legacy,
        CompressionType::None,
        false, // big endian
        true,  // explicit VR
        false, // no encapsulation
    ),
    
    // =============================================================================
    // JPEG BASELINE AND EXTENDED
    // =============================================================================
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.4.50",
        "JPEG Baseline (Process 1)",
        TransferSyntaxCategory::LossyCompressed,
        CompressionType::JPEG,
        true, // little endian
        true, // explicit VR
        true, // encapsulated
    ),
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.4.51",
        "JPEG Extended (Process 2 & 4)",
        TransferSyntaxCategory::LossyCompressed,
        CompressionType::JPEG,
        true, // little endian
        true, // explicit VR
        true, // encapsulated
    ),
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.4.52",
        "JPEG Extended (Process 3 & 5) (Retired)",
        TransferSyntaxCategory::Legacy,
        CompressionType::JPEG,
        true, // little endian
        true, // explicit VR
        true, // encapsulated
    ),
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.4.53",
        "JPEG Spectral Selection, Non-Hierarchical (Process 6 & 8) (Retired)",
        TransferSyntaxCategory::Legacy,
        CompressionType::JPEG,
        true, // little endian
        true, // explicit VR
        true, // encapsulated
    ),
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.4.54",
        "JPEG Spectral Selection, Non-Hierarchical (Process 7 & 9) (Retired)",
        TransferSyntaxCategory::Legacy,
        CompressionType::JPEG,
        true, // little endian
        true, // explicit VR
        true, // encapsulated
    ),
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.4.55",
        "JPEG Full Progression, Non-Hierarchical (Process 10 & 12) (Retired)",
        TransferSyntaxCategory::Legacy,
        CompressionType::JPEG,
        true, // little endian
        true, // explicit VR
        true, // encapsulated
    ),
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.4.56",
        "JPEG Full Progression, Non-Hierarchical (Process 11 & 13) (Retired)",
        TransferSyntaxCategory::Legacy,
        CompressionType::JPEG,
        true, // little endian
        true, // explicit VR
        true, // encapsulated
    ),
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.4.57",
        "JPEG Lossless, Non-Hierarchical (Process 14)",
        TransferSyntaxCategory::LosslessCompressed,
        CompressionType::JPEGLossless,
        true, // little endian
        true, // explicit VR
        true, // encapsulated
    ),
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.4.58",
        "JPEG Lossless, Non-Hierarchical (Process 15) (Retired)",
        TransferSyntaxCategory::Legacy,
        CompressionType::JPEGLossless,
        true, // little endian
        true, // explicit VR
        true, // encapsulated
    ),
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.4.59",
        "JPEG Extended, Hierarchical (Process 16 & 18) (Retired)",
        TransferSyntaxCategory::Legacy,
        CompressionType::JPEG,
        true, // little endian
        true, // explicit VR
        true, // encapsulated
    ),
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.4.60",
        "JPEG Extended, Hierarchical (Process 17 & 19) (Retired)",
        TransferSyntaxCategory::Legacy,
        CompressionType::JPEG,
        true, // little endian
        true, // explicit VR
        true, // encapsulated
    ),
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.4.61",
        "JPEG Spectral Selection, Hierarchical (Process 20 & 22) (Retired)",
        TransferSyntaxCategory::Legacy,
        CompressionType::JPEG,
        true, // little endian
        true, // explicit VR
        true, // encapsulated
    ),
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.4.62",
        "JPEG Spectral Selection, Hierarchical (Process 21 & 23) (Retired)",
        TransferSyntaxCategory::Legacy,
        CompressionType::JPEG,
        true, // little endian
        true, // explicit VR
        true, // encapsulated
    ),
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.4.63",
        "JPEG Full Progression, Hierarchical (Process 24 & 26) (Retired)",
        TransferSyntaxCategory::Legacy,
        CompressionType::JPEG,
        true, // little endian
        true, // explicit VR
        true, // encapsulated
    ),
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.4.64",
        "JPEG Full Progression, Hierarchical (Process 25 & 27) (Retired)",
        TransferSyntaxCategory::Legacy,
        CompressionType::JPEG,
        true, // little endian
        true, // explicit VR
        true, // encapsulated
    ),
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.4.65",
        "JPEG Lossless, Hierarchical (Process 28) (Retired)",
        TransferSyntaxCategory::Legacy,
        CompressionType::JPEGLossless,
        true, // little endian
        true, // explicit VR
        true, // encapsulated
    ),
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.4.66",
        "JPEG Lossless, Hierarchical (Process 29) (Retired)",
        TransferSyntaxCategory::Legacy,
        CompressionType::JPEGLossless,
        true, // little endian
        true, // explicit VR
        true, // encapsulated
    ),
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.4.70",
        "JPEG Lossless, Non-Hierarchical, First-Order Prediction (Process 14 [Selection Value 1])",
        TransferSyntaxCategory::LosslessCompressed,
        CompressionType::JPEGLossless,
        true, // little endian
        true, // explicit VR
        true, // encapsulated
    ),
    
    // =============================================================================
    // JPEG-LS
    // =============================================================================
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.4.80",
        "JPEG-LS Lossless Image Compression",
        TransferSyntaxCategory::LosslessCompressed,
        CompressionType::JPEGLS,
        true, // little endian
        true, // explicit VR
        true, // encapsulated
    ),
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.4.81",
        "JPEG-LS Lossy (Near-Lossless) Image Compression",
        TransferSyntaxCategory::LossyCompressed,
        CompressionType::JPEGLS,
        true, // little endian
        true, // explicit VR
        true, // encapsulated
    ),
    
    // =============================================================================
    // JPEG 2000
    // =============================================================================
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.4.90",
        "JPEG 2000 Image Compression (Lossless Only)",
        TransferSyntaxCategory::LosslessCompressed,
        CompressionType::JPEG2000,
        true, // little endian
        true, // explicit VR
        true, // encapsulated
    ),
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.4.91",
        "JPEG 2000 Image Compression",
        TransferSyntaxCategory::LossyCompressed,
        CompressionType::JPEG2000,
        true, // little endian
        true, // explicit VR
        true, // encapsulated
    ),
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.4.92",
        "JPEG 2000 Part 2 Multi-component Image Compression (Lossless Only)",
        TransferSyntaxCategory::LosslessCompressed,
        CompressionType::JPEG2000,
        true, // little endian
        true, // explicit VR
        true, // encapsulated
    ),
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.4.93",
        "JPEG 2000 Part 2 Multi-component Image Compression",
        TransferSyntaxCategory::LossyCompressed,
        CompressionType::JPEG2000,
        true, // little endian
        true, // explicit VR
        true, // encapsulated
    ),
    
    // =============================================================================
    // JPIP (JPEG 2000 Interactive Protocol)
    // =============================================================================
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.4.94",
        "JPIP Referenced",
        TransferSyntaxCategory::Legacy,
        CompressionType::JPEG2000,
        true, // little endian
        true, // explicit VR
        true, // encapsulated
    ),
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.4.95",
        "JPIP Referenced Deflate",
        TransferSyntaxCategory::Legacy,
        CompressionType::JPEG2000,
        true, // little endian
        true, // explicit VR
        true, // encapsulated
    ),
    
    // =============================================================================
    // RLE LOSSLESS
    // =============================================================================
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.5",
        "RLE Lossless",
        TransferSyntaxCategory::LosslessCompressed,
        CompressionType::RLE,
        true, // little endian
        true, // explicit VR
        true, // encapsulated
    ),
    
    // =============================================================================
    // RFC 2557 MIME ENCAPSULATION (Retired)
    // =============================================================================
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.6.1",
        "RFC 2557 MIME encapsulation (Retired)",
        TransferSyntaxCategory::Legacy,
        CompressionType::None,
        true, // little endian
        true, // explicit VR
        false, // no encapsulation
    ),
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.6.2",
        "XML Encoding (Retired)",
        TransferSyntaxCategory::Legacy,
        CompressionType::None,
        true, // little endian
        true, // explicit VR
        false, // no encapsulation
    ),
    
    // =============================================================================
    // SMPTE ST 2110-20 UNCOMPRESSED PROGRESSIVE ACTIVE VIDEO
    // =============================================================================
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.7.1",
        "SMPTE ST 2110-20 Uncompressed Progressive Active Video",
        TransferSyntaxCategory::Video,
        CompressionType::None,
        true, // little endian
        true, // explicit VR
        true, // encapsulated
    ),
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.7.2",
        "SMPTE ST 2110-20 Uncompressed Interlaced Active Video",
        TransferSyntaxCategory::Video,
        CompressionType::None,
        true, // little endian
        true, // explicit VR
        true, // encapsulated
    ),
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.7.3",
        "SMPTE ST 2110-30 PCM Digital Audio",
        TransferSyntaxCategory::Video,
        CompressionType::None,
        true, // little endian
        true, // explicit VR
        true, // encapsulated
    ),
    
    // =============================================================================
    // MPEG2 AND MPEG4 VIDEO COMPRESSION
    // =============================================================================
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.4.100",
        "MPEG2 Main Profile / Main Level",
        TransferSyntaxCategory::Video,
        CompressionType::MPEG2,
        true, // little endian
        true, // explicit VR
        true, // encapsulated
    ),
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.4.101",
        "MPEG2 Main Profile / High Level",
        TransferSyntaxCategory::Video,
        CompressionType::MPEG2,
        true, // little endian
        true, // explicit VR
        true, // encapsulated
    ),
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.4.102",
        "MPEG-4 AVC/H.264 High Profile / Level 4.1",
        TransferSyntaxCategory::Video,
        CompressionType::H264,
        true, // little endian
        true, // explicit VR
        true, // encapsulated
    ),
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.4.103",
        "MPEG-4 AVC/H.264 BD-compatible High Profile / Level 4.1",
        TransferSyntaxCategory::Video,
        CompressionType::H264,
        true, // little endian
        true, // explicit VR
        true, // encapsulated
    ),
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.4.104",
        "MPEG-4 AVC/H.264 High Profile / Level 4.2 For 2D Video",
        TransferSyntaxCategory::Video,
        CompressionType::H264,
        true, // little endian
        true, // explicit VR
        true, // encapsulated
    ),
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.4.105",
        "MPEG-4 AVC/H.264 High Profile / Level 4.2 For 3D Video",
        TransferSyntaxCategory::Video,
        CompressionType::H264,
        true, // little endian
        true, // explicit VR
        true, // encapsulated
    ),
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.4.106",
        "MPEG-4 AVC/H.264 Stereo High Profile / Level 4.2",
        TransferSyntaxCategory::Video,
        CompressionType::H264,
        true, // little endian
        true, // explicit VR
        true, // encapsulated
    ),
    
    // =============================================================================
    // HEVC/H.265 VIDEO COMPRESSION
    // =============================================================================
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.4.107",
        "HEVC/H.265 Main Profile / Level 5.1",
        TransferSyntaxCategory::Video,
        CompressionType::H265,
        true, // little endian
        true, // explicit VR
        true, // encapsulated
    ),
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.4.108",
        "HEVC/H.265 Main 10 Profile / Level 5.1",
        TransferSyntaxCategory::Video,
        CompressionType::H265,
        true, // little endian
        true, // explicit VR
        true, // encapsulated
    ),
    
    // =============================================================================
    // HIGH THROUGHPUT JPEG 2000
    // =============================================================================
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.4.201",
        "High-Throughput JPEG 2000 Image Compression (Lossless Only)",
        TransferSyntaxCategory::LosslessCompressed,
        CompressionType::JPEG2000,
        true, // little endian
        true, // explicit VR
        true, // encapsulated
    ),
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.4.202",
        "High-Throughput JPEG 2000 with RPCL Options Image Compression (Lossless Only)",
        TransferSyntaxCategory::LosslessCompressed,
        CompressionType::JPEG2000,
        true, // little endian
        true, // explicit VR
        true, // encapsulated
    ),
    
    TransferSyntaxInfo::new(
        "1.2.840.10008.1.2.4.203",
        "High-Throughput JPEG 2000 Image Compression",
        TransferSyntaxCategory::LossyCompressed,
        CompressionType::JPEG2000,
        true, // little endian
        true, // explicit VR
        true, // encapsulated
    ),
];

/// Get transfer syntaxes appropriate for different use cases
pub fn get_basic_transfer_syntaxes() -> Vec<&'static str> {
    vec![
        "1.2.840.10008.1.2.1", // Explicit VR Little Endian
        "1.2.840.10008.1.2",   // Implicit VR Little Endian
    ]
}

pub fn get_lossless_transfer_syntaxes() -> Vec<&'static str> {
    vec![
        "1.2.840.10008.1.2.1",   // Explicit VR Little Endian
        "1.2.840.10008.1.2",     // Implicit VR Little Endian
        "1.2.840.10008.1.2.4.70", // JPEG Lossless
        "1.2.840.10008.1.2.4.80", // JPEG-LS Lossless
        "1.2.840.10008.1.2.4.90", // JPEG 2000 Lossless
        "1.2.840.10008.1.2.5",    // RLE Lossless
    ]
}

pub fn get_compressed_transfer_syntaxes() -> Vec<&'static str> {
    vec![
        "1.2.840.10008.1.2.1",   // Explicit VR Little Endian
        "1.2.840.10008.1.2",     // Implicit VR Little Endian
        "1.2.840.10008.1.2.4.50", // JPEG Baseline
        "1.2.840.10008.1.2.4.51", // JPEG Extended
        "1.2.840.10008.1.2.4.70", // JPEG Lossless
        "1.2.840.10008.1.2.4.80", // JPEG-LS Lossless
        "1.2.840.10008.1.2.4.81", // JPEG-LS Near-Lossless
        "1.2.840.10008.1.2.4.90", // JPEG 2000 Lossless
        "1.2.840.10008.1.2.4.91", // JPEG 2000
        "1.2.840.10008.1.2.5",    // RLE Lossless
    ]
}

pub fn get_comprehensive_transfer_syntaxes() -> Vec<&'static str> {
    let registry = TransferSyntaxRegistry::new();
    registry.get_all_uids()
}

pub fn get_video_transfer_syntaxes() -> Vec<&'static str> {
    let registry = TransferSyntaxRegistry::new();
    registry.get_by_category(TransferSyntaxCategory::Video)
        .iter()
        .map(|ts| ts.uid)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transfer_syntax_registry() {
        let registry = TransferSyntaxRegistry::new();
        
        // Test basic lookups
        assert!(registry.is_supported("1.2.840.10008.1.2"));
        assert!(registry.is_supported("1.2.840.10008.1.2.1"));
        assert!(!registry.is_supported("invalid.uid"));
        
        // Test name lookup
        assert_eq!(
            registry.get_name("1.2.840.10008.1.2"),
            Some("Implicit VR Little Endian")
        );
        
        // Test compression detection
        assert!(!registry.is_compressed("1.2.840.10008.1.2"));
        assert!(registry.is_compressed("1.2.840.10008.1.2.4.50"));
        
        // Test that we have a comprehensive list
        let all_uids = registry.get_all_uids();
        assert!(all_uids.len() > 30); // Should have many transfer syntaxes
    }
    
    #[test]
    fn test_transfer_syntax_categories() {
        let basic = get_basic_transfer_syntaxes();
        let lossless = get_lossless_transfer_syntaxes();
        let compressed = get_compressed_transfer_syntaxes();
        let comprehensive = get_comprehensive_transfer_syntaxes();
        
        assert!(!basic.is_empty());
        assert!(lossless.len() > basic.len());
        assert!(compressed.len() > lossless.len());
        assert!(comprehensive.len() > compressed.len());
        
        // Basic should be subset of others
        for ts in &basic {
            assert!(lossless.contains(ts));
            assert!(compressed.contains(ts));
            assert!(comprehensive.contains(ts));
        }
    }
    
    #[test]
    fn test_transfer_syntax_properties() {
        let registry = TransferSyntaxRegistry::new();
        
        // Test specific properties
        let explicit_vr = registry.get("1.2.840.10008.1.2.1").unwrap();
        assert!(explicit_vr.is_explicit_vr);
        assert!(explicit_vr.is_little_endian);
        assert!(!explicit_vr.is_compressed());
        
        let jpeg_baseline = registry.get("1.2.840.10008.1.2.4.50").unwrap();
        assert!(jpeg_baseline.is_compressed());
        assert!(!jpeg_baseline.is_lossless());
        assert!(jpeg_baseline.supports_encapsulation);
    }
}
