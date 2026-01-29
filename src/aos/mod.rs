//! Sequential implementation of secret sharing operations.
//! 
//! This module provides a single-threaded implementation suitable for
//! smaller datasets or environments without parallelization support.

use ark_ff::{PrimeField, BigInteger, BigInt};
use ark_std::rand::thread_rng;
use ldpc_toolbox::gf2::GF2;
use ndarray::{Array1, Array2};
use num_traits::{One, Zero};
use indicatif::ProgressBar;

use crate::types::{SecretParams, Shares, CodeInitParams, ReconstructMetrics, DecodingStats};
use crate::code::AdditiveCode;
use crate::code::ldpc_impl::LdpcCode;
use crate::aos_core::{self, ExecutionStrategy};

pub mod utils;

/// Sequential execution strategy marker type.
pub struct SequentialStrategy;

impl ExecutionStrategy for SequentialStrategy {
    fn generate_random_vec<F: PrimeField>(len: usize) -> Vec<F> {
        let mut rng = thread_rng();
        (0..len).map(|_| F::rand(&mut rng)).collect()
    }

    fn dot_product<F: PrimeField>(a: &[F], b: &[F]) -> F {
        utils::dot_product(a, b)
    }

    fn create_message_matrix<F: PrimeField>(
        r_vec: &[F],
        nrows: usize,
        ncols: usize,
        progress_bar: &ProgressBar
    ) -> Array2<GF2> {
        let mut message_matrix = Array2::<GF2>::from_elem((nrows, ncols), GF2::zero());
        
        // Pre-allocate GF2 instances to avoid repeated allocation
        let gf2_zero = GF2::zero();
        let gf2_one = GF2::one();

        // Use chunk processing to improve cache locality
        const CHUNK_SIZE: usize = 16;
        for chunk_start in (0..ncols).step_by(CHUNK_SIZE) {
            let chunk_end = std::cmp::min(chunk_start + CHUNK_SIZE, ncols);
            
            for i in chunk_start..chunk_end {
                let val_int = r_vec[i].into_bigint();
                let bits = val_int.to_bits_le();
                
                // Direct assignment with optimized bounds checking
                for (j, &b) in bits.iter().enumerate().take(nrows) {
                    message_matrix[(j, i)] = if b { gf2_one } else { gf2_zero };
                }
                
                // Fill remaining bits with zero if necessary
                for j in bits.len()..nrows {
                    message_matrix[(j, i)] = gf2_zero;
                }
                
                progress_bar.inc(1);
            }
        }
        
        message_matrix
    }

    fn encode_rows(
        message_matrix: &Array2<GF2>,
        code_impl: &LdpcCode,
        nrows: usize,
        output_cols: usize,
        progress_bar: &ProgressBar
    ) -> Array2<GF2> {
        let mut encoded_matrix = Array2::<GF2>::from_elem((nrows, output_cols), GF2::zero());
        
        for i in 0..nrows {
            let encoded = code_impl.encode(&message_matrix.row(i).to_owned());
            encoded_matrix.row_mut(i).assign(&encoded);
            progress_bar.inc(1);
        }
        
        encoded_matrix
    }

    fn decode_rows(
        encoded_matrix: &Array2<GF2>,
        code_impl: &LdpcCode,
        present_columns: &[bool],
        input_length: usize,
        nrows: usize,
        progress_bar: &ProgressBar
    ) -> (Array2<GF2>, DecodingStats) {
        let mut decoded_matrix = Array2::<GF2>::from_elem((nrows, input_length), GF2::zero());
        let mut successful_rows = 0;
        let mut failed_rows = 0;
        let mut total_iterations = 0;
        let mut max_iterations_hit = 0;
        let max_iter_limit = code_impl.max_iterations();

        let update_interval = std::time::Duration::from_millis(200);
        let mut last_update_time = std::time::Instant::now();

        for i in 0..nrows {
            let row_input = encoded_matrix.row(i).to_owned();
            
            let decode_result = code_impl.decode(&row_input, present_columns);
            
            // Track iterations
            total_iterations += decode_result.iterations;
            if decode_result.iterations >= max_iter_limit {
                max_iterations_hit += 1;
            }
            
            if decode_result.success {
                successful_rows += 1;
                let gf2_vec: Vec<GF2> = decode_result.codeword
                    .into_iter()
                    .take(input_length)
                    .map(|bit| if bit == 1 { GF2::one() } else { GF2::zero() })
                    .collect();
                let gf2_array = Array1::from(gf2_vec);
                decoded_matrix.row_mut(i).assign(&gf2_array);
            } else {
                failed_rows += 1;
            }

            progress_bar.inc(1);

            // Update progress message periodically
            if last_update_time.elapsed() >= update_interval {
                progress_bar.set_message(format!(
                    "success rate: {:.2}% ({} ok, {} failed)",
                    (successful_rows as f64 / (i + 1) as f64) * 100.0,
                    successful_rows,
                    failed_rows
                ));
                last_update_time = std::time::Instant::now();
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
        progress_bar: &ProgressBar
    ) -> Vec<F> {
        let mut r = vec![F::zero(); input_length];
        
        for i in 0..input_length {
            let bool_vec: Vec<bool> = decoded_matrix.column(i)
                .iter()
                .map(|&x| x.is_one())
                .collect();
            let big_int = BigInteger::from_bits_le(&bool_vec);
            r[i] = F::from_bigint(big_int).expect("Failed to convert BigInt to field element");
            progress_bar.inc(1);
        }
        
        r
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