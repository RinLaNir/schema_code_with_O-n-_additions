use crate::code::AdditiveCode;
use ark_ff::Field;
use ldpc_toolbox::gf2::GF2;
use ldpc_toolbox::decoder::factory::DecoderImplementation;
use ldpc_toolbox::codes::ccsds::{AR4JARate, AR4JAInfoSize};
use ndarray::Array1;
use serde::{Serialize, Deserialize, Serializer};
use std::fmt::Debug;
use std::time::Duration;

// ── Serde helpers ──────────────────────────────────────────────

/// Serde module for Duration fields — use as `#[serde(with = "duration_as_ms")]`.
pub mod duration_as_ms {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S: Serializer>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_f64(duration.as_secs_f64() * 1000.0)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Duration, D::Error> {
        let ms = f64::deserialize(deserializer)?;
        Ok(Duration::from_secs_f64(ms / 1000.0))
    }
}

/// Serialize Duration as milliseconds (f64). For serialize-only fields use `#[serde(serialize_with = "serialize_duration_as_ms")]`.
pub fn serialize_duration_as_ms<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    duration_as_ms::serialize(duration, serializer)
}

// Serializes `Option<T: Debug>` as its debug string, or null if None.
fn serialize_option_debug<T: Debug, S: Serializer>(val: &Option<T>, serializer: S) -> Result<S::Ok, S::Error> {
    match val {
        Some(v) => serializer.serialize_str(&format!("{:?}", v)),
        None => serializer.serialize_none(),
    }
}

// ── Enum parsing ───────────────────────────────────────────────

/// All 36 decoder variants with their canonical string names.
const DECODER_VARIANTS: &[(DecoderImplementation, &str)] = &[
    (DecoderImplementation::Phif64, "Phif64"),
    (DecoderImplementation::Phif32, "Phif32"),
    (DecoderImplementation::Tanhf64, "Tanhf64"),
    (DecoderImplementation::Tanhf32, "Tanhf32"),
    (DecoderImplementation::Minstarapproxf64, "Minstarapproxf64"),
    (DecoderImplementation::Minstarapproxf32, "Minstarapproxf32"),
    (DecoderImplementation::Minstarapproxi8, "Minstarapproxi8"),
    (DecoderImplementation::Minstarapproxi8Jones, "Minstarapproxi8Jones"),
    (DecoderImplementation::Minstarapproxi8PartialHardLimit, "Minstarapproxi8PartialHardLimit"),
    (DecoderImplementation::Minstarapproxi8JonesPartialHardLimit, "Minstarapproxi8JonesPartialHardLimit"),
    (DecoderImplementation::Minstarapproxi8Deg1Clip, "Minstarapproxi8Deg1Clip"),
    (DecoderImplementation::Minstarapproxi8JonesDeg1Clip, "Minstarapproxi8JonesDeg1Clip"),
    (DecoderImplementation::Minstarapproxi8PartialHardLimitDeg1Clip, "Minstarapproxi8PartialHardLimitDeg1Clip"),
    (DecoderImplementation::Minstarapproxi8JonesPartialHardLimitDeg1Clip, "Minstarapproxi8JonesPartialHardLimitDeg1Clip"),
    (DecoderImplementation::Aminstarf64, "Aminstarf64"),
    (DecoderImplementation::Aminstarf32, "Aminstarf32"),
    (DecoderImplementation::Aminstari8, "Aminstari8"),
    (DecoderImplementation::Aminstari8Jones, "Aminstari8Jones"),
    (DecoderImplementation::Aminstari8PartialHardLimit, "Aminstari8PartialHardLimit"),
    (DecoderImplementation::Aminstari8JonesPartialHardLimit, "Aminstari8JonesPartialHardLimit"),
    (DecoderImplementation::Aminstari8Deg1Clip, "Aminstari8Deg1Clip"),
    (DecoderImplementation::Aminstari8JonesDeg1Clip, "Aminstari8JonesDeg1Clip"),
    (DecoderImplementation::Aminstari8PartialHardLimitDeg1Clip, "Aminstari8PartialHardLimitDeg1Clip"),
    (DecoderImplementation::Aminstari8JonesPartialHardLimitDeg1Clip, "Aminstari8JonesPartialHardLimitDeg1Clip"),
    (DecoderImplementation::HLPhif64, "HLPhif64"),
    (DecoderImplementation::HLPhif32, "HLPhif32"),
    (DecoderImplementation::HLTanhf64, "HLTanhf64"),
    (DecoderImplementation::HLTanhf32, "HLTanhf32"),
    (DecoderImplementation::HLMinstarapproxf64, "HLMinstarapproxf64"),
    (DecoderImplementation::HLMinstarapproxf32, "HLMinstarapproxf32"),
    (DecoderImplementation::HLMinstarapproxi8, "HLMinstarapproxi8"),
    (DecoderImplementation::HLMinstarapproxi8PartialHardLimit, "HLMinstarapproxi8PartialHardLimit"),
    (DecoderImplementation::HLAminstarf64, "HLAminstarf64"),
    (DecoderImplementation::HLAminstarf32, "HLAminstarf32"),
    (DecoderImplementation::HLAminstari8, "HLAminstari8"),
    (DecoderImplementation::HLAminstari8PartialHardLimit, "HLAminstari8PartialHardLimit"),
];

/// Returns all decoder variants with their canonical string names.
pub fn decoder_variants() -> &'static [(DecoderImplementation, &'static str)] {
    DECODER_VARIANTS
}

/// Returns all decoder implementation variants.
pub fn all_decoder_types() -> Vec<DecoderImplementation> {
    DECODER_VARIANTS.iter().map(|(d, _)| *d).collect()
}

/// Parse a decoder type string (e.g. `"Aminstarf32"`) into `DecoderImplementation`.
pub fn parse_decoder_type(s: &str) -> Result<DecoderImplementation, String> {
    DECODER_VARIANTS.iter()
        .find(|(_, name)| *name == s)
        .map(|(d, _)| *d)
        .ok_or_else(|| format!("Unknown decoder type: {}", s))
}

/// Parse an LDPC rate string (e.g. `"R1_2"`, `"1_2"`) into `AR4JARate`.
pub fn parse_ldpc_rate(s: &str) -> Result<AR4JARate, String> {
    match s {
        "R1_2" | "1_2" => Ok(AR4JARate::R1_2),
        "R2_3" | "2_3" => Ok(AR4JARate::R2_3),
        "R4_5" | "4_5" => Ok(AR4JARate::R4_5),
        _ => Err(format!("Unknown LDPC rate: {}", s)),
    }
}

/// Parse an LDPC info size string (e.g. `"K1024"`) into `AR4JAInfoSize`.
pub fn parse_ldpc_info_size(s: &str) -> Result<AR4JAInfoSize, String> {
    match s {
        "K1024" => Ok(AR4JAInfoSize::K1024),
        "K4096" => Ok(AR4JAInfoSize::K4096),
        "K16384" => Ok(AR4JAInfoSize::K16384),
        _ => Err(format!("Unknown LDPC info size: {}", s)),
    }
}

/// Returns the number of information bits for a given info size.
pub fn info_bits(info_size: AR4JAInfoSize) -> usize {
    match info_size {
        AR4JAInfoSize::K1024 => 1024,
        AR4JAInfoSize::K4096 => 4096,
        AR4JAInfoSize::K16384 => 16384,
    }
}

#[derive(Clone, Serialize)]
pub struct CodeInitParams {
    #[serde(serialize_with = "serialize_option_debug")]
    pub decoder_type: Option<DecoderImplementation>,
    #[serde(serialize_with = "serialize_option_debug")]
    pub ldpc_rate: Option<AR4JARate>,
    #[serde(serialize_with = "serialize_option_debug")]
    pub ldpc_info_size: Option<AR4JAInfoSize>,
    pub max_iterations: Option<usize>,
    pub llr_value: Option<f64>,
}

/// Performance metrics for an operation phase
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PhaseMetrics {
    /// Phase label; retained for debugging.
    #[allow(dead_code)]
    #[serde(default)]
    pub name: String,
    #[serde(with = "duration_as_ms", default)]
    pub duration: Duration,
    #[serde(default)]
    pub percentage: f64,
}

/// Performance metrics for the deal operation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DealMetrics {
    pub rand_vec_generation: PhaseMetrics,
    pub dot_product: PhaseMetrics,
    pub matrix_creation: PhaseMetrics,
    pub encoding: PhaseMetrics,
    pub share_creation: PhaseMetrics,
    /// Total time - kept for debugging and completeness
    #[allow(dead_code)]
    #[serde(with = "duration_as_ms", default)]
    pub total_time: Duration,
}

/// Statistics for LDPC decoding operations
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DecodingStats {
    pub total_rows: usize,
    pub successful_rows: usize,
    pub failed_rows: usize,
    pub total_iterations: usize,
    pub avg_iterations: f64,
    pub max_iterations_hit: usize,
}

impl DecodingStats {
    pub fn new(total_rows: usize, successful_rows: usize, failed_rows: usize, 
               total_iterations: usize, max_iterations_hit: usize) -> Self {
        let avg_iterations = if successful_rows > 0 {
            total_iterations as f64 / successful_rows as f64
        } else {
            0.0
        };
        
        DecodingStats {
            total_rows,
            successful_rows,
            failed_rows,
            total_iterations,
            avg_iterations,
            max_iterations_hit,
        }
    }
    
    pub fn success_rate(&self) -> f64 {
        if self.total_rows > 0 {
            self.successful_rows as f64 / self.total_rows as f64
        } else {
            0.0
        }
    }
}

/// Performance metrics for the reconstruct operation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReconstructMetrics {
    pub matrix_setup: PhaseMetrics,
    pub row_decoding: PhaseMetrics,
    pub field_reconstruction: PhaseMetrics, 
    pub final_computation: PhaseMetrics,
    /// Total time - kept for debugging and completeness
    #[allow(dead_code)]
    #[serde(with = "duration_as_ms", default)]
    pub total_time: Duration,
    /// Decoding statistics (iterations, success rate, etc.)
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub decoding_stats: Option<DecodingStats>,
}

/// Metrics for parallel execution efficiency
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ParallelMetrics {
    pub thread_count: usize,
    pub speedup: Option<f64>,
    pub efficiency: Option<f64>,
}

/// Throughput metrics for performance analysis
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThroughputMetrics {
    pub shares_per_second: f64,
    pub bits_per_second: f64,
}

impl PhaseMetrics {
    pub fn new(name: &str, duration: Duration, total_time: Duration) -> Self {
        let total_nanos = total_time.as_nanos();
        let percentage = if total_nanos > 0 {
            duration.as_nanos() as f64 / total_nanos as f64 * 100.0
        } else {
            0.0
        };

        PhaseMetrics {
            name: name.to_string(),
            duration,
            percentage,
        }
    }
}

pub struct CodeParams<C: AdditiveCode> {
    pub output_length: u32,
    pub input_length: u32,
    pub code_impl: C
}

pub struct SecretParams<C: AdditiveCode, F: Field> {
    pub code: CodeParams<C>,
    pub a: Vec<F>,
}

pub struct Shares<F: Field> {
    pub shares: Vec<Share>,
    pub z0: F,
    pub metrics: Option<DealMetrics>,
}

pub struct Share {
    pub y: Array1<GF2>,
    pub i: u32,
}