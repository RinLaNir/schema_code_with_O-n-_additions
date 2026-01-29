use crate::code::AdditiveCode;
use ark_ff::Field;
use ldpc_toolbox::gf2::GF2;
use ldpc_toolbox::decoder::factory::DecoderImplementation;
use ldpc_toolbox::codes::ccsds::{AR4JARate, AR4JAInfoSize};
use ndarray::Array1;
use serde::{Serialize, Deserialize, Serializer, Deserializer};
use std::time::Duration;

/// Helper function to serialize Duration as milliseconds (f64)
fn serialize_duration_as_ms<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_f64(duration.as_secs_f64() * 1000.0)
}

/// Helper function to deserialize Duration from milliseconds (f64)
fn deserialize_duration_from_ms<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let ms = f64::deserialize(deserializer)?;
    Ok(Duration::from_secs_f64(ms / 1000.0))
}

/// Helper to serialize DecoderImplementation as string
fn serialize_decoder_type<S>(decoder: &Option<DecoderImplementation>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match decoder {
        Some(d) => serializer.serialize_str(&format!("{:?}", d)),
        None => serializer.serialize_none(),
    }
}

/// Helper to serialize AR4JARate as string
fn serialize_ldpc_rate<S>(rate: &Option<AR4JARate>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match rate {
        Some(r) => serializer.serialize_str(&format!("{:?}", r)),
        None => serializer.serialize_none(),
    }
}

/// Helper to serialize AR4JAInfoSize as string
fn serialize_ldpc_info_size<S>(size: &Option<AR4JAInfoSize>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match size {
        Some(s) => serializer.serialize_str(&format!("{:?}", s)),
        None => serializer.serialize_none(),
    }
}

#[derive(Clone, Serialize)]
pub struct CodeInitParams {
    #[serde(serialize_with = "serialize_decoder_type")]
    pub decoder_type: Option<DecoderImplementation>,
    #[serde(serialize_with = "serialize_ldpc_rate")]
    pub ldpc_rate: Option<AR4JARate>,
    #[serde(serialize_with = "serialize_ldpc_info_size")]
    pub ldpc_info_size: Option<AR4JAInfoSize>,
    pub max_iterations: Option<usize>,
    pub llr_value: Option<f64>,
}

/// Performance metrics for an operation phase
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PhaseMetrics {
    /// Phase name - kept for debugging and future display features
    #[allow(dead_code)]
    #[serde(default)]
    pub name: String,
    #[serde(serialize_with = "serialize_duration_as_ms", deserialize_with = "deserialize_duration_from_ms", default)]
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
    #[serde(serialize_with = "serialize_duration_as_ms", deserialize_with = "deserialize_duration_from_ms", default)]
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
    #[serde(serialize_with = "serialize_duration_as_ms", deserialize_with = "deserialize_duration_from_ms", default)]
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
        let percentage = if total_time.as_nanos() > 0 {
            (duration.as_nanos() as f64 / total_time.as_nanos() as f64) * 100.0
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