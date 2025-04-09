use ldpc_toolbox::codes::ccsds::{AR4JACode, AR4JARate, AR4JAInfoSize};
use ldpc_toolbox::encoder::Encoder;
use ldpc_toolbox::gf2::GF2;
use ldpc_toolbox::decoder::DecoderOutput;
use ndarray::Array1;
use num_traits::One;
use crate::code::AdditiveCode;
use crate::types::CodeInitParams;
use ldpc_toolbox::decoder::factory::{DecoderFactory, DecoderImplementation};
use ldpc_toolbox::decoder::factory::DecoderImplementation::Aminstarf32;

pub struct LdpcCode {
    code: AR4JACode,
    encoder: Encoder,
    arithmetic: DecoderImplementation,
    max_iterations: usize,
    llr_value: f64,
    input_length: usize,
    output_length: usize,
}

impl AdditiveCode for LdpcCode {
    fn setup(params: CodeInitParams) -> Self {
        let rate = params.ldpc_rate.unwrap_or(AR4JARate::R4_5);
        let info_size = params.ldpc_info_size.unwrap_or(AR4JAInfoSize::K1024);
        let arithmetic = params.decoder_type.unwrap_or(Aminstarf32);
        let max_iterations = params.max_iterations.unwrap_or(300);
        let llr_value = params.llr_value.unwrap_or(1.3863);
        let h = AR4JACode::new(rate, info_size).h();
        let input_length = match (rate, info_size) {
            (AR4JARate::R1_2, AR4JAInfoSize::K1024) => 1024,
            (AR4JARate::R2_3, AR4JAInfoSize::K1024) => 1024,
            (AR4JARate::R4_5, AR4JAInfoSize::K1024) => 1024,
            (AR4JARate::R1_2, AR4JAInfoSize::K4096) => 4096,
            (AR4JARate::R2_3, AR4JAInfoSize::K4096) => 4096,
            (AR4JARate::R4_5, AR4JAInfoSize::K4096) => 4096,
            (AR4JARate::R1_2, AR4JAInfoSize::K16384) => 16384,
            (AR4JARate::R2_3, AR4JAInfoSize::K16384) => 16384,
            (AR4JARate::R4_5, AR4JAInfoSize::K16384) => 16384,
        };
        let output_length = h.num_cols();
        let code = AR4JACode::new(rate, info_size);
        let encoder = Encoder::from_h(&h).unwrap();

        LdpcCode { 
            code, 
            encoder, 
            arithmetic, 
            max_iterations,
            llr_value,
            input_length,
            output_length,
        }
    }

    fn encode(&self, message: &Array1<GF2>) -> Array1<GF2> {
        self.encoder.encode(message)
    }

    fn decode(&self, input: &Array1<GF2>, present_positions: &[bool]) -> Result<DecoderOutput, DecoderOutput> {
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
                    -self.llr_value // LLR for bit 1
                } else {
                    self.llr_value  // LLR for bit 0
                }
            })
            .collect();
        
        let mut decoder = self.arithmetic.build_decoder(self.code.h());
        decoder.decode(message.as_slice(), self.max_iterations)
    }

    fn input_length(&self) -> u32 {
        self.input_length as u32
    }

    fn output_length(&self) -> u32 {
        self.output_length as u32
    }
}