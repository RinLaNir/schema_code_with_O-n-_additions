pub mod ldpc_impl;

use ldpc_toolbox::decoder::DecoderOutput;
use ldpc_toolbox::gf2::GF2;
use ndarray::Array1;

pub trait AdditiveCode {
    fn setup(params: crate::types::CodeInitParams) -> Self;
    fn encode(&self, input: &Array1<GF2>) -> Array1<GF2>;
    fn decode(&mut self, input: &Array1<GF2>, present_positions: &[bool]) -> Result<DecoderOutput, DecoderOutput>;
    fn input_length(&self) -> u32;
    fn output_length(&self) -> u32;
}