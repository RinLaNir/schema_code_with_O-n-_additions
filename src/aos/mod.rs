//! Sequential implementation of secret sharing operations.

use ldpc_toolbox::gf2::GF2;
use ndarray::Array2;
use num_traits::{One, Zero};

use crate::aos_core::{self, codeword_to_gf2_buf, ExecutionStrategy};
use crate::code::ldpc_impl::LdpcCode;
use crate::code::AdditiveCode;
use crate::types::{
    CodeInitParams, DecodingStats, F2PowElement, ReconstructMetrics, SecretParams, Shares,
};

pub struct SequentialStrategy;

impl ExecutionStrategy for SequentialStrategy {
    fn generate_random_columns(len: usize, bit_len: usize) -> Vec<F2PowElement> {
        let mut rng = rand::rng();
        (0..len)
            .map(|_| F2PowElement::random(bit_len, &mut rng))
            .collect()
    }

    fn create_message_matrix(columns: &[F2PowElement], nrows: usize, ncols: usize) -> Array2<GF2> {
        let mut message_matrix = Array2::<GF2>::from_elem((nrows, ncols), GF2::zero());
        let gf2_zero = GF2::zero();
        let gf2_one = GF2::one();

        for (col_idx, column) in columns.iter().enumerate().take(ncols) {
            for row_idx in 0..nrows {
                message_matrix[(row_idx, col_idx)] = if column.bit(row_idx) {
                    gf2_one
                } else {
                    gf2_zero
                };
            }
        }

        message_matrix
    }

    fn encode_rows(
        message_matrix: &Array2<GF2>,
        code_impl: &LdpcCode,
        nrows: usize,
        output_cols: usize,
    ) -> Array2<GF2> {
        let mut encoded_matrix = Array2::<GF2>::from_elem((nrows, output_cols), GF2::zero());

        for i in 0..nrows {
            let encoded = code_impl.encode(&message_matrix.row(i).to_owned());
            encoded_matrix.row_mut(i).assign(&encoded);
        }

        encoded_matrix
    }

    fn decode_rows(
        encoded_matrix: &Array2<GF2>,
        code_impl: &LdpcCode,
        present_columns: &[bool],
        input_length: usize,
        nrows: usize,
    ) -> (Array2<GF2>, DecodingStats) {
        let mut decoded_matrix = Array2::<GF2>::from_elem((nrows, input_length), GF2::zero());
        let mut successful_rows = 0;
        let mut failed_rows = 0;
        let mut total_iterations = 0;
        let mut max_iterations_hit = 0;
        let max_iter_limit = code_impl.max_iterations();

        for i in 0..nrows {
            let row_input = encoded_matrix.row(i).to_owned();
            let decode_result = code_impl.decode(&row_input, present_columns);

            total_iterations += decode_result.iterations;
            if decode_result.iterations >= max_iter_limit {
                max_iterations_hit += 1;
            }

            if decode_result.success {
                successful_rows += 1;
                let mut row_mut = decoded_matrix.row_mut(i);
                codeword_to_gf2_buf(
                    &decode_result.codeword,
                    row_mut.as_slice_mut().unwrap(),
                    input_length,
                );
            } else {
                failed_rows += 1;
            }
        }

        (
            decoded_matrix,
            DecodingStats::new(
                nrows,
                successful_rows,
                failed_rows,
                total_iterations,
                max_iterations_hit,
            ),
        )
    }

    fn reconstruct_columns(
        decoded_matrix: &Array2<GF2>,
        input_length: usize,
        bit_len: usize,
    ) -> Vec<F2PowElement> {
        (0..input_length)
            .map(|col_idx| {
                let mut column = F2PowElement::zero(bit_len);
                for row_idx in 0..bit_len {
                    if decoded_matrix[(row_idx, col_idx)].is_one() {
                        column.set_bit(row_idx, true);
                    }
                }
                column
            })
            .collect()
    }
}

pub fn setup(params: CodeInitParams) -> SecretParams<LdpcCode> {
    aos_core::setup(params)
}

pub fn deal(pp: &SecretParams<LdpcCode>, secret: &F2PowElement) -> Shares {
    aos_core::deal_with_strategy::<SequentialStrategy>(pp, secret)
}

pub fn reconstruct(
    pp: &SecretParams<LdpcCode>,
    shares: &Shares,
) -> (Option<F2PowElement>, Option<ReconstructMetrics>) {
    aos_core::reconstruct_with_strategy::<SequentialStrategy>(pp, shares)
}
