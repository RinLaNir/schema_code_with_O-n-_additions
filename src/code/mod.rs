pub mod ldpc_impl;

use ldpc_toolbox::decoder::DecoderOutput;
use ldpc_toolbox::gf2::GF2;
use ndarray::Array1;

#[derive(Debug, Clone)]
pub struct DecodeResult {
    pub codeword: Vec<u8>,
    pub iterations: usize,
    pub success: bool,
}

impl DecodeResult {
    pub fn from_decoder_output(output: DecoderOutput, success: bool) -> Self {
        DecodeResult {
            codeword: output.codeword,
            iterations: output.iterations,
            success,
        }
    }
}

pub trait AdditiveCode {
    fn setup(params: crate::types::CodeInitParams) -> Self;
    fn encode(&self, input: &Array1<GF2>) -> Array1<GF2>;
    fn decode(&self, input: &Array1<GF2>, present_positions: &[bool]) -> DecodeResult;
    fn input_length(&self) -> u32;
    fn output_length(&self) -> u32;
    fn max_iterations(&self) -> usize;
}