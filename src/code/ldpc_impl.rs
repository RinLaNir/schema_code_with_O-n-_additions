use rand::SeedableRng;
use rand::rngs::StdRng;
use ldpc_toolbox::codes::ccsds::{AR4JACode, AR4JARate, AR4JAInfoSize};
use ldpc_toolbox::encoder::Encoder;
use ldpc_toolbox::decoder::arithmetic::Aminstarf32;
use ldpc_toolbox::decoder::horizontal_layered::Decoder;
use ldpc_toolbox::gf2::GF2;
use ark_ff::Field;
use ldpc_toolbox::decoder::DecoderOutput;
use ldpc_toolbox::sparse::SparseMatrix;
use ndarray::{s, Array1, Array2, ArrayBase, Data, Ix1};
use num_traits::One;
use crate::code::AdditiveCode;
use crate::types::CodeInitParams;

pub struct LdpcCode {
    code: AR4JACode,
    decoder: Decoder<Aminstarf32>,
    encoder: Encoder,
}

impl AdditiveCode for LdpcCode {
    fn setup(params: CodeInitParams) -> Self {
        let code = AR4JACode::new(AR4JARate::R4_5, AR4JAInfoSize::K1024);
        let encoder = Encoder::from_h(&code.h()).unwrap();
        let decoder = Decoder::new(code.h(), Aminstarf32::new());

        LdpcCode { code, decoder, encoder }
    }
    
    fn encode(&self, message: &Array1<GF2>) -> Array1<GF2> {
        self.encoder.encode(message)
    }

    fn decode(&mut self, input: &Array1<GF2>) -> Result<DecoderOutput, DecoderOutput> {
        let max_iterations = 200;
        let message: Vec<f64> = input
            .iter()
            .map(|&elem| if elem.is_one() { -1.3863 } else { 1.3863 })
            .collect();
        self.decoder.decode(message.as_slice(), max_iterations)
    }
    
    fn generator_matrix(&self) -> SparseMatrix {
        self.code.h()
    }

    // Temporary hardcoded
    fn input_length(&self) -> u32 {
        1024
    }

    // Temporary hardcoded
    fn output_length(&self) -> u32 {
        1408
    }
}