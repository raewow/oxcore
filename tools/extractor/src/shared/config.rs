//! Extractor Configuration
//!
//! Configuration options for controlling extraction and optimization behavior.

/// Extraction flags (bit flags)
pub const EXTRACT_MAP: u32 = 1;
pub const EXTRACT_DBC: u32 = 2;
pub const EXTRACT_CAMERA: u32 = 4;

/// Extractor configuration
#[derive(Debug, Clone)]
pub struct ExtractorConfig {
    /// Enable height limiting (minimum height cutoff)
    pub allow_height_limit: bool,

    /// Minimum height value to use (below this is set to this value)
    pub use_min_height: f32,

    /// Enable float-to-integer compression for height data
    pub allow_float_to_int: bool,

    /// Maximum height difference for u8 compression (2.0 meters)
    pub float_to_int8_limit: f32,

    /// Maximum height difference for u16 compression (2048 meters)
    pub float_to_int16_limit: f32,

    /// If height difference is less than this, consider surface flat
    pub flat_height_delta_limit: f32,

    /// If liquid height difference is less than this, consider it flat
    pub flat_liquid_delta_limit: f32,
}

impl Default for ExtractorConfig {
    fn default() -> Self {
        Self {
            allow_height_limit: true,
            use_min_height: -500.0,
            allow_float_to_int: false, // Disabled by default for accuracy
            float_to_int8_limit: 2.0,
            float_to_int16_limit: 2048.0,
            flat_height_delta_limit: 0.005,
            flat_liquid_delta_limit: 0.001,
        }
    }
}

impl ExtractorConfig {
    /// Create a new configuration with float compression enabled
    pub fn with_compression() -> Self {
        Self {
            allow_float_to_int: true,
            ..Default::default()
        }
    }

    /// Create a configuration optimized for accuracy (no compression)
    pub fn accurate() -> Self {
        Self {
            allow_float_to_int: false,
            allow_height_limit: false,
            ..Default::default()
        }
    }
}
