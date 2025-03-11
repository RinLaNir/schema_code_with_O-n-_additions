use crate::code::AdditiveCode;
use ark_ff::Field;
use ldpc_toolbox::gf2::GF2;
use ldpc_toolbox::decoder::factory::DecoderImplementation;
use ldpc_toolbox::codes::ccsds::{AR4JARate, AR4JAInfoSize};
use ndarray::Array1;
use std::time::Duration;

pub struct CodeInitParams {
    pub decoder_type: Option<DecoderImplementation>,
    pub ldpc_rate: Option<AR4JARate>,
    pub ldpc_info_size: Option<AR4JAInfoSize>,
    pub max_iterations: Option<usize>,
    pub llr_value: Option<f64>,
}

/// Performance metrics for an operation phase
#[derive(Debug, Clone, Default)]
pub struct PhaseMetrics {
    pub name: String,
    pub duration: Duration,
    pub percentage: f64,
}

/// Performance metrics for the deal operation
#[derive(Debug, Clone, Default)]
pub struct DealMetrics {
    pub rand_vec_generation: PhaseMetrics,
    pub dot_product: PhaseMetrics,
    pub matrix_creation: PhaseMetrics,
    pub encoding: PhaseMetrics,
    pub share_creation: PhaseMetrics,
    pub total_time: Duration,
}

/// Performance metrics for the reconstruct operation
#[derive(Debug, Clone, Default)]
pub struct ReconstructMetrics {
    pub matrix_setup: PhaseMetrics,
    pub row_decoding: PhaseMetrics,
    pub field_reconstruction: PhaseMetrics, 
    pub final_computation: PhaseMetrics,
    pub total_time: Duration,
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