use crate::code::AdditiveCode;
use ark_ff::Field;
use ldpc_toolbox::gf2::GF2;
use ldpc_toolbox::decoder::factory::DecoderImplementation;
use ldpc_toolbox::codes::ccsds::{AR4JARate, AR4JAInfoSize};
use ndarray::Array1;

pub struct CodeInitParams {
    pub decoder_type: Option<DecoderImplementation>,
    pub ldpc_rate: Option<AR4JARate>,
    pub ldpc_info_size: Option<AR4JAInfoSize>,
    pub max_iterations: Option<usize>,
    pub llr_value: Option<f64>,
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
}

pub struct Share {
    pub y: Array1<GF2>,
    pub i: u32,
}