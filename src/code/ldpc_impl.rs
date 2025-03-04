use ldpc_toolbox::codes::ccsds::{AR4JACode, AR4JARate, AR4JAInfoSize};
use ldpc_toolbox::encoder::Encoder;
use ldpc_toolbox::decoder::arithmetic::Aminstarf32;
use ldpc_toolbox::decoder::horizontal_layered::Decoder;
use ldpc_toolbox::gf2::GF2;
use ldpc_toolbox::decoder::DecoderOutput;
use ndarray::Array1;
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
    
    fn decode(&mut self, input: &Array1<GF2>, present_positions: &[bool]) -> Result<DecoderOutput, DecoderOutput> {
        let max_iterations = 100;
        
        // Check if the input vector and present_positions array have the same dimensions
        assert_eq!(input.len(), present_positions.len(), 
            "Input length ({}) must match present_positions length ({})", 
            input.len(), present_positions.len());
        
        let message: Vec<f64> = input
            .iter()
            .zip(present_positions.iter())
            .map(|(&elem, &is_present)| {
                if !is_present {
                    0.0 // LLR = 0 for erased bits (complete uncertainty)
                } else if elem.is_one() {
                    -1.3863 // LLR for bit 1
                } else {
                    1.3863  // LLR for bit 0
                }
            })
            .collect();
        
        self.decoder.decode(message.as_slice(), max_iterations)
    }

    fn input_length(&self) -> u32 {
        1024
    }

    fn output_length(&self) -> u32 {
        1408
    }
}