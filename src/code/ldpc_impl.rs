use ldpc_toolbox::codes::ccsds::{AR4JACode, AR4JARate, AR4JAInfoSize};
use ldpc_toolbox::encoder::Encoder;
use ldpc_toolbox::gf2::GF2;
use ndarray::Array1;
use num_traits::One;
use crate::code::{AdditiveCode, DecodeResult};
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

    fn decode(&self, input: &Array1<GF2>, present_positions: &[bool]) -> DecodeResult {
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
                    -self.llr_value
                } else {
                    self.llr_value
                }
            })
            .collect();
        
        let mut decoder = self.arithmetic.build_decoder(self.code.h());
        match decoder.decode(message.as_slice(), self.max_iterations) {
            Ok(output) => DecodeResult::from_decoder_output(output, true),
            Err(output) => DecodeResult::from_decoder_output(output, false),
        }
    }

    fn input_length(&self) -> u32 {
        self.input_length as u32
    }

    fn output_length(&self) -> u32 {
        self.output_length as u32
    }

    fn max_iterations(&self) -> usize {
        self.max_iterations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ldpc_toolbox::codes::ccsds::{AR4JARate, AR4JAInfoSize};
    use ldpc_toolbox::decoder::factory::DecoderImplementation;
    use num_traits::Zero;

    fn create_test_code() -> LdpcCode {
        let params = CodeInitParams {
            decoder_type: Some(DecoderImplementation::Aminstarf32),
            ldpc_rate: Some(AR4JARate::R4_5),
            ldpc_info_size: Some(AR4JAInfoSize::K1024),
            max_iterations: Some(300),
            llr_value: Some(1.3863),
        };
        LdpcCode::setup(params)
    }

    #[test]
    fn test_ldpc_setup_dimensions() {
        let code = create_test_code();
        
        assert_eq!(code.input_length(), 1024, "Input length should be 1024 for K1024");
        assert!(code.output_length() > code.input_length(), 
            "Output length should be greater than input length");
    }

    #[test]
    fn test_ldpc_encode_output_length() {
        let code = create_test_code();
        
        let message = Array1::from(vec![GF2::zero(); code.input_length() as usize]);
        let encoded = code.encode(&message);
        
        assert_eq!(encoded.len(), code.output_length() as usize,
            "Encoded length should match output_length");
    }

    #[test]
    fn test_ldpc_encode_zero_message() {
        let code = create_test_code();
        
        let message = Array1::from(vec![GF2::zero(); code.input_length() as usize]);
        let encoded = code.encode(&message);
        
        for bit in encoded.iter() {
            assert!(bit.is_zero(), "Encoding zero message should produce zero codeword");
        }
    }

    #[test]
    fn test_ldpc_encode_decode_roundtrip_no_erasures() {
        let code = create_test_code();
        
        let mut message_vec = vec![GF2::zero(); code.input_length() as usize];
        message_vec[0] = GF2::one();
        message_vec[10] = GF2::one();
        message_vec[100] = GF2::one();
        let message = Array1::from(message_vec.clone());
        
        let encoded = code.encode(&message);
        
        let present_positions = vec![true; code.output_length() as usize];
        
        let decode_result = code.decode(&encoded, &present_positions);
        
        assert!(decode_result.success, "Decoding should succeed with no erasures");
        
        for (i, &original_bit) in message_vec.iter().enumerate() {
            let decoded_bit = if decode_result.codeword[i] == 1 { GF2::one() } else { GF2::zero() };
            assert_eq!(original_bit, decoded_bit, 
                "Decoded bit {} should match original", i);
        }
    }

    #[test]
    fn test_ldpc_decode_with_small_erasures() {
        let code = create_test_code();
        
        let mut message_vec = vec![GF2::zero(); code.input_length() as usize];
        for i in (0..100).step_by(10) {
            message_vec[i] = GF2::one();
        }
        let message = Array1::from(message_vec.clone());
        
        let encoded = code.encode(&message);
        
        let mut present_positions = vec![true; code.output_length() as usize];
        for i in (0..50).step_by(5) {
            present_positions[i] = false;
        }
        
        let decode_result = code.decode(&encoded, &present_positions);
        
        assert!(decode_result.success, "Decoding should succeed with small erasures");
    }

    #[test]
    fn test_ldpc_different_rates() {
        let rates = [AR4JARate::R1_2, AR4JARate::R2_3, AR4JARate::R4_5];
        
        for rate in rates.iter() {
            let params = CodeInitParams {
                decoder_type: Some(DecoderImplementation::Aminstarf32),
                ldpc_rate: Some(*rate),
                ldpc_info_size: Some(AR4JAInfoSize::K1024),
                max_iterations: Some(100),
                llr_value: Some(1.3863),
            };
            
            let code = LdpcCode::setup(params);
            
            assert_eq!(code.input_length(), 1024);
            assert!(code.output_length() > code.input_length());
        }
    }
}