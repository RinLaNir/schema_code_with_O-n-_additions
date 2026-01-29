//! Parallel implementation of secret sharing operations using Rayon.
//! 
//! This module provides a multi-threaded implementation optimized for
//! larger datasets utilizing all available CPU cores.

use ark_ff::{PrimeField, BigInteger, BigInt};
use ark_std::rand::thread_rng;
use ldpc_toolbox::gf2::GF2;
use ndarray::{Array1, Array2};
use num_traits::{One, Zero};
use indicatif::ProgressBar;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use rayon::prelude::*;

use crate::types::{SecretParams, Shares, CodeInitParams, ReconstructMetrics, DecodingStats};
use crate::code::AdditiveCode;
use crate::code::ldpc_impl::LdpcCode;
use crate::aos_core::{self, ExecutionStrategy};

pub mod utils;

/// Parallel execution strategy marker type using Rayon.
pub struct ParallelStrategy;

impl ExecutionStrategy for ParallelStrategy {
    fn generate_random_vec<F: PrimeField>(len: usize) -> Vec<F> {
        (0..len)
            .into_par_iter()
            .map(|_| {
                let mut thread_rng = thread_rng();
                F::rand(&mut thread_rng)
            })
            .collect()
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
        // Build columns in parallel, then assemble the matrix
        let columns: Vec<Vec<GF2>> = (0..ncols)
            .into_par_iter()
            .map(|i| {
                let val_int = r_vec[i].into_bigint();
                let mut bits: Vec<bool> = val_int.to_bits_le();
                bits.resize(nrows, false);

                let col: Vec<GF2> = bits
                    .iter()
                    .map(|&b| if b { GF2::one() } else { GF2::zero() })
                    .collect();

                progress_bar.inc(1);
                col
            })
            .collect();

        Array2::from_shape_fn((nrows, ncols), |(r, c)| columns[c][r])
    }

    fn encode_rows(
        message_matrix: &Array2<GF2>,
        code_impl: &LdpcCode,
        nrows: usize,
        output_cols: usize,
        progress_bar: &ProgressBar
    ) -> Array2<GF2> {
        // Encode each row in parallel, then assemble
        let encoded_rows: Vec<Array1<GF2>> = (0..nrows)
            .into_par_iter()
            .map(|i| {
                let row = message_matrix.row(i).to_owned();
                let encoded = code_impl.encode(&row);
                progress_bar.inc(1);
                encoded
            })
            .collect();

        // Flatten rows into a single buffer (row-major) and build the matrix
        let mut flat: Vec<GF2> = Vec::with_capacity(nrows * output_cols);
        for row in encoded_rows {
            flat.extend(row.into_iter());
        }
        
        Array2::from_shape_vec((nrows, output_cols), flat)
            .expect("Encoded matrix shape mismatch")
    }

    fn decode_rows(
        encoded_matrix: &Array2<GF2>,
        code_impl: &LdpcCode,
        present_columns: &[bool],
        input_length: usize,
        nrows: usize,
        progress_bar: &ProgressBar
    ) -> (Array2<GF2>, DecodingStats) {
        // Atomic counters for decoding statistics
        let successful_rows = Arc::new(AtomicUsize::new(0));
        let failed_rows = Arc::new(AtomicUsize::new(0));
        let total_iterations = Arc::new(AtomicUsize::new(0));
        let max_iterations_hit = Arc::new(AtomicUsize::new(0));
        let max_iter_limit = code_impl.max_iterations();

        progress_bar.enable_steady_tick(std::time::Duration::from_millis(200));

        // Structure to store decoded row results
        struct DecodedRow {
            row_index: usize,
            gf2_array: Option<Array1<GF2>>,
        }

        let present_columns = Arc::new(present_columns.to_vec());

        // Parallel decoding with proper error handling
        let decoded_rows: Vec<DecodedRow> = (0..nrows)
            .into_par_iter()
            .map(|i| {
                let row_input = encoded_matrix.row(i).to_owned();
                let decode_result = code_impl.decode(&row_input, &present_columns);
                
                // Track iterations
                total_iterations.fetch_add(decode_result.iterations, Ordering::Relaxed);
                if decode_result.iterations >= max_iter_limit {
                    max_iterations_hit.fetch_add(1, Ordering::Relaxed);
                }
                
                if decode_result.success {
                    let gf2_vec: Vec<GF2> = decode_result.codeword
                        .into_iter()
                        .take(input_length)
                        .map(|bit| if bit == 1 { GF2::one() } else { GF2::zero() })
                        .collect();
                    successful_rows.fetch_add(1, Ordering::Relaxed);
                    DecodedRow { 
                        row_index: i, 
                        gf2_array: Some(Array1::from(gf2_vec)) 
                    }
                } else {
                    failed_rows.fetch_add(1, Ordering::Relaxed);
                    DecodedRow { 
                        row_index: i, 
                        gf2_array: None 
                    }
                }
            })
            .collect();

        // Assemble decoded matrix from results
        let mut decoded_storage: Vec<GF2> = vec![GF2::zero(); nrows * input_length];
        for decoded_row in &decoded_rows {
            if let Some(ref row_data) = decoded_row.gf2_array {
                let row_start = decoded_row.row_index * input_length;
                for (offset, v) in row_data.iter().enumerate() {
                    decoded_storage[row_start + offset] = *v;
                }
            }
            progress_bar.inc(1);
        }

        let decoded_matrix = Array2::from_shape_vec((nrows, input_length), decoded_storage)
            .expect("Decoded matrix shape mismatch");

        let success_count = successful_rows.load(Ordering::Relaxed);
        let fail_count = failed_rows.load(Ordering::Relaxed);
        let iter_total = total_iterations.load(Ordering::Relaxed);
        let max_hit = max_iterations_hit.load(Ordering::Relaxed);

        let decoding_stats = DecodingStats::new(
            nrows,
            success_count,
            fail_count,
            iter_total,
            max_hit,
        );

        (decoded_matrix, decoding_stats)
    }

    fn reconstruct_field_elements<F: PrimeField<BigInt = BigInt<4>>>(
        decoded_matrix: &Array2<GF2>,
        input_length: usize,
        progress_bar: &ProgressBar
    ) -> Vec<F> {
        // Optimized chunk size for cache line efficiency
        const CHUNK_SIZE: usize = 32;
        
        (0..input_length)
            .into_par_iter()
            .with_min_len(CHUNK_SIZE)
            .map(|i| {
                let bool_vec: Vec<bool> = decoded_matrix.column(i)
                    .iter()
                    .map(|&x| x.is_one())
                    .collect();
                let big_int = BigInteger::from_bits_le(&bool_vec);
                let val = F::from_bigint(big_int).expect("Failed to convert BigInt to field element");
                progress_bar.inc(1);
                val
            })
            .collect()
    }
}

/// Setup the secret sharing scheme parameters.
/// 
/// Delegates to the shared implementation in aos_core.
pub fn setup<F: PrimeField>(params: CodeInitParams, c: u32) -> SecretParams<LdpcCode, F> {
    aos_core::setup(params, c)
}

/// Deal a secret into shares using parallel processing.
pub fn deal<F: PrimeField>(pp: &SecretParams<LdpcCode, F>, s: F) -> Shares<F> {
    aos_core::deal_with_strategy::<F, ParallelStrategy>(pp, s)
}

/// Reconstruct a secret from shares using parallel processing.
pub fn reconstruct<F: PrimeField<BigInt = BigInt<4>>>(
    pp: &SecretParams<LdpcCode, F>, 
    shares: &Shares<F>
) -> (F, Option<ReconstructMetrics>) {
    aos_core::reconstruct_with_strategy::<F, ParallelStrategy>(pp, shares)
}