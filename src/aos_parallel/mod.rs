//! Parallel implementation of secret sharing operations using Rayon.
//! 
//! This module provides a multi-threaded implementation optimized for
//! larger datasets utilizing all available CPU cores.

use ark_ff::{PrimeField, BigInteger, BigInt};
use ldpc_toolbox::gf2::GF2;
use ndarray::{Array1, Array2};
use num_traits::{One, Zero};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use rayon::prelude::*;

use crate::types::{SecretParams, Shares, CodeInitParams, ReconstructMetrics, DecodingStats};
use crate::code::AdditiveCode;
use crate::code::ldpc_impl::LdpcCode;
use crate::aos_core::{self, ExecutionStrategy, codeword_to_gf2_buf, column_to_field_element};

/// Parallel execution strategy marker type using Rayon.
pub struct ParallelStrategy;

impl ExecutionStrategy for ParallelStrategy {
    fn generate_random_vec<F: PrimeField>(len: usize) -> Vec<F> {
        (0..len)
            .into_par_iter()
            .map(|_| {
                let mut thread_rng = ark_std::rand::thread_rng();
                F::rand(&mut thread_rng)
            })
            .collect()
    }

    fn dot_product<F: PrimeField>(a: &[F], b: &[F]) -> F {
        aos_core::dot_product_adaptive(a, b)
    }

    fn create_message_matrix<F: PrimeField>(
        r_vec: &[F],
        nrows: usize,
        ncols: usize,
    ) -> Array2<GF2> {
        let columns: Vec<Vec<GF2>> = (0..ncols)
            .into_par_iter()
            .map(|i| {
                let val_int = r_vec[i].into_bigint();
                let mut bits: Vec<bool> = val_int.to_bits_le();
                bits.resize(nrows, false);

                bits.iter()
                    .map(|&b| if b { GF2::one() } else { GF2::zero() })
                    .collect()
            })
            .collect();

        // Transpose columns to row-major flat buffer — avoids from_shape_fn random access
        let mut flat: Vec<GF2> = Vec::with_capacity(nrows * ncols);
        for r in 0..nrows {
            for col in columns.iter() {
                flat.push(col[r]);
            }
        }
        Array2::from_shape_vec((nrows, ncols), flat)
            .expect("Message matrix shape mismatch")
    }

    fn encode_rows(
        message_matrix: &Array2<GF2>,
        code_impl: &LdpcCode,
        nrows: usize,
        output_cols: usize,
    ) -> Array2<GF2> {
        let encoded_rows: Vec<Array1<GF2>> = (0..nrows)
            .into_par_iter()
            .map(|i| {
                let row = message_matrix.row(i).to_owned();
                code_impl.encode(&row)
            })
            .collect();

        let mut flat: Vec<GF2> = Vec::with_capacity(nrows * output_cols);
        for row in &encoded_rows {
            flat.extend(row.iter().copied());
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
    ) -> (Array2<GF2>, DecodingStats) {
        let successful_rows = Arc::new(AtomicUsize::new(0));
        let failed_rows = Arc::new(AtomicUsize::new(0));
        let total_iterations = Arc::new(AtomicUsize::new(0));
        let max_iterations_hit = Arc::new(AtomicUsize::new(0));
        let max_iter_limit = code_impl.max_iterations();

        let decoded_rows: Vec<(usize, Option<Vec<GF2>>)> = (0..nrows)
            .into_par_iter()
            .map(|i| {
                let row_input = encoded_matrix.row(i).to_owned();
                let decode_result = code_impl.decode(&row_input, present_columns);

                total_iterations.fetch_add(decode_result.iterations, Ordering::Relaxed);
                if decode_result.iterations >= max_iter_limit {
                    max_iterations_hit.fetch_add(1, Ordering::Relaxed);
                }

                if decode_result.success {
                    successful_rows.fetch_add(1, Ordering::Relaxed);
                    let mut gf2_buf = vec![GF2::zero(); input_length];
                    codeword_to_gf2_buf(&decode_result.codeword, &mut gf2_buf, input_length);
                    (i, Some(gf2_buf))
                } else {
                    failed_rows.fetch_add(1, Ordering::Relaxed);
                    (i, None)
                }
            })
            .collect();

        let mut decoded_storage: Vec<GF2> = vec![GF2::zero(); nrows * input_length];
        for (row_idx, row_data) in &decoded_rows {
            if let Some(buf) = row_data {
                let row_start = row_idx * input_length;
                decoded_storage[row_start..row_start + input_length].copy_from_slice(buf);
            }
        }

        let decoded_matrix = Array2::from_shape_vec((nrows, input_length), decoded_storage)
            .expect("Decoded matrix shape mismatch");

        let decoding_stats = DecodingStats::new(
            nrows,
            successful_rows.load(Ordering::Relaxed),
            failed_rows.load(Ordering::Relaxed),
            total_iterations.load(Ordering::Relaxed),
            max_iterations_hit.load(Ordering::Relaxed),
        );

        (decoded_matrix, decoding_stats)
    }

    fn reconstruct_field_elements<F: PrimeField<BigInt = BigInt<4>>>(
        decoded_matrix: &Array2<GF2>,
        input_length: usize,
    ) -> Vec<F> {
        const CHUNK_SIZE: usize = 32;

        (0..input_length)
            .into_par_iter()
            .with_min_len(CHUNK_SIZE)
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