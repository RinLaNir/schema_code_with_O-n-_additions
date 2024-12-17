use rand::SeedableRng;
use rand::rngs::StdRng;
use ldpc::codes::LinearCode;
use ldpc::decoders::{BpDecoder, LinearDecoder};
use ldpc::noise::Probability;
use sparse_bin_mat::{SparseBinMat, SparseBinSlice, SparseBinVec};

use crate::code::AdditiveCode;
use crate::types::CodeInitParams;

pub struct LdpcCode {
    code: LinearCode,
    decoder: BpDecoder,
    k: u32,
}

impl AdditiveCode for LdpcCode {
    fn setup(params: CodeInitParams) -> Self {
        let code = LinearCode::random_regular_code()
            .num_bits(params.num_bits)
            .num_checks(params.num_checks)
            .bit_degree(params.bit_degree)
            .check_degree(params.check_degree)
            .sample_with(&mut StdRng::seed_from_u64(123)) // TODO: Make seed flexible
            .unwrap();

        let k = code.generator_matrix().number_of_rows() as u32;
        let decoder = BpDecoder::new(code.parity_check_matrix(), Probability::new(0.1), 10);

        LdpcCode {
            code,
            decoder,
            k
        }
    }

    fn encode(&self, input: &SparseBinMat) -> SparseBinMat {
        input * self.code.generator_matrix()
    }

    fn decode(&self, input: SparseBinSlice) -> SparseBinVec {
        let right_pos = input.len();
        let left_pos = right_pos - self.k() as usize;
        let to_keep: Vec<usize> = (left_pos..right_pos).collect();

        let decoded = self.decoder.decode(input);
        decoded.keep_only_positions(&to_keep).unwrap()
    }

    fn generator_matrix(&self) -> &SparseBinMat {
        self.code.generator_matrix()
    }

    fn k(&self) -> u32 {
        self.k
    }
}
