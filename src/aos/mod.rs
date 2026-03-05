//! Sequential implementation of secret sharing operations.
//! 
//! This module provides a single-threaded implementation suitable for
//! smaller datasets or environments without parallelization support.

use ark_ff::{PrimeField, BigInteger, BigInt};
use ldpc_toolbox::gf2::GF2;
use ndarray::Array2;
use num_traits::{One, Zero};

use crate::types::{SecretParams, Shares, CodeInitParams, ReconstructMetrics, DecodingStats};
use crate::code::AdditiveCode;
use crate::code::ldpc_impl::LdpcCode;
use crate::aos_core::{self, ExecutionStrategy, codeword_to_gf2_buf, column_to_field_element};

pub struct SequentialStrategy;

impl ExecutionStrategy for SequentialStrategy {
    fn generate_random_vec<F: PrimeField>(len: usize) -> Vec<F> {
        let mut rng = ark_std::rand::thread_rng();
        (0..len).map(|_| F::rand(&mut rng)).collect()
    }

    fn dot_product<F: PrimeField>(a: &[F], b: &[F]) -> F {
        aos_core::dot_product_sequential(a, b)
    }

    fn create_message_matrix<F: PrimeField>(
        r_vec: &[F],
        nrows: usize,
        ncols: usize,
    ) -> Array2<GF2> {
        let mut message_matrix = Array2::<GF2>::from_elem((nrows, ncols), GF2::zero());
        let gf2_zero = GF2::zero();
        let gf2_one = GF2::one();

        for (i, r) in r_vec.iter().enumerate().take(ncols) {
            let bits = r.into_bigint().to_bits_le();
            for (j, &b) in bits.iter().enumerate().take(nrows) {
                message_matrix[(j, i)] = if b { gf2_one } else { gf2_zero };
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
                codeword_to_gf2_buf(&decode_result.codeword, row_mut.as_slice_mut().unwrap(), input_length);
            } else {
                failed_rows += 1;
            }
        }

        let decoding_stats = DecodingStats::new(
            nrows,
            successful_rows,
            failed_rows,
            total_iterations,
            max_iterations_hit,
        );

        (decoded_matrix, decoding_stats)
    }

    fn reconstruct_field_elements<F: PrimeField<BigInt = BigInt<4>>>(
        decoded_matrix: &Array2<GF2>,
        input_length: usize,
    ) -> Vec<F> {
        (0..input_length)
            .map(|i| column_to_field_element::<F>(decoded_matrix, i))
            .collect()
    }
}

/// Setup the secret sharing scheme parameters.
/// 
/// Delegates to the shared implementation in aos_core.
pub fn setup<F: PrimeField>(params: CodeInitParams, c: u32) -> SecretParams<LdpcCode, F> {
    aos_core::setup(params, c)
}

/// Deal a secret into shares using sequential processing.
pub fn deal<F: PrimeField>(pp: &SecretParams<LdpcCode, F>, s: F) -> Shares<F> {
    aos_core::deal_with_strategy::<F, SequentialStrategy>(pp, s)
}

/// Reconstruct a secret from shares using sequential processing.
pub fn reconstruct<F: PrimeField<BigInt = BigInt<4>>>(
    pp: &SecretParams<LdpcCode, F>, 
    shares: &Shares<F>
) -> (F, Option<ReconstructMetrics>) {
    aos_core::reconstruct_with_strategy::<F, SequentialStrategy>(pp, shares)
}