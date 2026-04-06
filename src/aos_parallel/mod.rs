//! Parallel implementation of secret sharing operations using Rayon.

use ldpc_toolbox::gf2::GF2;
use ndarray::{Array1, Array2};
use num_traits::{One, Zero};
use rayon::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::aos_core::{self, codeword_to_gf2_buf, ExecutionStrategy};
use crate::code::ldpc_impl::LdpcCode;
use crate::code::AdditiveCode;
use crate::types::{
    CodeInitParams, DecodingStats, F2PowElement, ReconstructMetrics, SecretParams, Shares,
};

pub struct ParallelStrategy;

impl ExecutionStrategy for ParallelStrategy {
    fn generate_random_columns(len: usize, bit_len: usize) -> Vec<F2PowElement> {
        (0..len)
            .into_par_iter()
            .map(|_| {
                let mut rng = rand::rng();
                F2PowElement::random(bit_len, &mut rng)
            })
            .collect()
    }

    fn create_message_matrix(columns: &[F2PowElement], nrows: usize, ncols: usize) -> Array2<GF2> {
        let columns: Vec<Vec<GF2>> = (0..ncols)
            .into_par_iter()
            .map(|col_idx| {
                (0..nrows)
                    .map(|row_idx| {
                        if columns[col_idx].bit(row_idx) {
                            GF2::one()
                        } else {
                            GF2::zero()
                        }
                    })
                    .collect()
            })
            .collect();

        let mut flat = Vec::with_capacity(nrows * ncols);
        for row_idx in 0..nrows {
            for column in &columns {
                flat.push(column[row_idx]);
            }
        }

        Array2::from_shape_vec((nrows, ncols), flat).expect("message matrix shape mismatch")
    }

    fn encode_rows(
        message_matrix: &Array2<GF2>,
        code_impl: &LdpcCode,
        nrows: usize,
        output_cols: usize,
    ) -> Array2<GF2> {
        let encoded_rows: Vec<Array1<GF2>> = (0..nrows)
            .into_par_iter()
            .map(|row_idx| code_impl.encode(&message_matrix.row(row_idx).to_owned()))
            .collect();

        let mut flat = Vec::with_capacity(nrows * output_cols);
        for row in &encoded_rows {
            flat.extend(row.iter().copied());
        }

        Array2::from_shape_vec((nrows, output_cols), flat).expect("encoded matrix shape mismatch")
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
            .map(|row_idx| {
                let row_input = encoded_matrix.row(row_idx).to_owned();
                let decode_result = code_impl.decode(&row_input, present_columns);

                total_iterations.fetch_add(decode_result.iterations, Ordering::Relaxed);
                if decode_result.iterations >= max_iter_limit {
                    max_iterations_hit.fetch_add(1, Ordering::Relaxed);
                }

                if decode_result.success {
                    successful_rows.fetch_add(1, Ordering::Relaxed);
                    let mut gf2_buf = vec![GF2::zero(); input_length];
                    codeword_to_gf2_buf(&decode_result.codeword, &mut gf2_buf, input_length);
                    (row_idx, Some(gf2_buf))
                } else {
                    failed_rows.fetch_add(1, Ordering::Relaxed);
                    (row_idx, None)
                }
            })
            .collect();

        let mut decoded_storage = vec![GF2::zero(); nrows * input_length];
        for (row_idx, row_data) in &decoded_rows {
            if let Some(buf) = row_data {
                let row_start = row_idx * input_length;
                decoded_storage[row_start..row_start + input_length].copy_from_slice(buf);
            }
        }

        let decoded_matrix = Array2::from_shape_vec((nrows, input_length), decoded_storage)
            .expect("decoded matrix shape mismatch");

        (
            decoded_matrix,
            DecodingStats::new(
                nrows,
                successful_rows.load(Ordering::Relaxed),
                failed_rows.load(Ordering::Relaxed),
                total_iterations.load(Ordering::Relaxed),
                max_iterations_hit.load(Ordering::Relaxed),
            ),
        )
    }

    fn reconstruct_columns(
        decoded_matrix: &Array2<GF2>,
        input_length: usize,
        bit_len: usize,
    ) -> Vec<F2PowElement> {
        const CHUNK_SIZE: usize = 32;

        (0..input_length)
            .into_par_iter()
            .with_min_len(CHUNK_SIZE)
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
    aos_core::deal_with_strategy::<ParallelStrategy>(pp, secret)
}

pub fn reconstruct(
    pp: &SecretParams<LdpcCode>,
    shares: &Shares,
) -> (Option<F2PowElement>, Option<ReconstructMetrics>) {
    aos_core::reconstruct_with_strategy::<ParallelStrategy>(pp, shares)
}
