//! Core module for secret sharing operations using LDPC codes.
//! 
//! This module contains shared logic for both sequential and parallel implementations.
//! It provides the `setup()` function and the `ExecutionStrategy` trait that defines
//! how different phases of deal/reconstruct operations are executed.

use ark_std::rand::Rng;
use ark_ff::{PrimeField, BigInt};
use ldpc_toolbox::gf2::GF2;
use ndarray::Array2;
use num_traits::{Zero, One};
use rayon::prelude::*;
use std::time::Instant;

use crate::types::{
    SecretParams, CodeParams, Shares, Share, CodeInitParams,
    PhaseMetrics, DealMetrics, ReconstructMetrics, DecodingStats
};
use crate::code::AdditiveCode;
use crate::code::ldpc_impl::LdpcCode;
use crate::{log_verbose, log_success};

/// Convert a decoded codeword (byte slice of 0/1 values) into GF2 elements,
/// writing directly into a pre-allocated output slice to avoid per-row allocations.
///
/// # Arguments
/// * `codeword` - Decoded byte slice where each byte is 0 or 1
/// * `out` - Pre-allocated mutable slice to write GF2 values into (must be `len` long)
/// * `len` - Number of elements to convert (takes first `len` bytes from `codeword`)
#[inline]
pub fn codeword_to_gf2_buf(codeword: &[u8], out: &mut [GF2], len: usize) {
    let gf2_one = GF2::one();
    let gf2_zero = GF2::zero();
    for (dst, &byte) in out[..len].iter_mut().zip(codeword.iter().take(len)) {
        *dst = if byte == 1 { gf2_one } else { gf2_zero };
    }
}

/// Convert a single column of a decoded GF2 matrix into a field element.
///
/// Shared logic for `reconstruct_field_elements` in both sequential and parallel strategies.
/// Reads a column, maps GF2 → bool, reconstructs via `BigInteger::from_bits_le`.
#[inline]
pub fn column_to_field_element<F: PrimeField<BigInt = BigInt<4>>>(
    decoded_matrix: &Array2<GF2>,
    col: usize,
) -> F {
    let bool_vec: Vec<bool> = decoded_matrix.column(col)
        .iter()
        .map(|x| x.is_one())
        .collect();
    let big_int = ark_ff::BigInteger::from_bits_le(&bool_vec);
    F::from_bigint(big_int).expect("Failed to convert BigInt to field element")
}

/// Compute dot product of two field-element slices strictly sequentially.
///
/// This function is used by the sequential execution strategy to keep
/// baseline measurements single-threaded.
pub fn dot_product_sequential<F: ark_ff::Field>(a: &[F], b: &[F]) -> F {
    a.iter().zip(b).fold(F::zero(), |acc, (x, y)| acc + (*x * *y))
}

/// Compute dot product with adaptive parallelism (parallel for ≥ 1024 elements).
pub fn dot_product_adaptive<F: ark_ff::Field + Send + Sync>(a: &[F], b: &[F]) -> F {
    const CHUNK_SIZE: usize = 1024;

    if a.len() < CHUNK_SIZE {
        return dot_product_sequential(a, b);
    }

    a.par_chunks(CHUNK_SIZE)
        .zip(b.par_chunks(CHUNK_SIZE))
        .map(|(ac, bc)| ac.iter().zip(bc).fold(F::zero(), |acc, (x, y)| acc + (*x * *y)))
        .reduce(|| F::zero(), |acc, x| acc + x)
}

/// Initialize the LDPC code and generate random coefficients.
///
/// # Arguments
/// * `params` - LDPC code initialization parameters
/// * `c` - Upper bound for random coefficient generation
/// 
/// # Returns
/// * `SecretParams` containing the code parameters and random coefficients
pub fn setup<F: PrimeField>(params: CodeInitParams, c: u32) -> SecretParams<LdpcCode, F> {
    let start_time = Instant::now();
    log_verbose!("Starting setup operation...");
    
    let code_impl = LdpcCode::setup(params);
    let input_length = code_impl.input_length();
    let output_length = code_impl.output_length();
    
    assert!(
        input_length >= F::MODULUS_BIT_SIZE, 
        "Number of bits ({}) must be greater than or equal to the modulus bit size ({})", 
        input_length, F::MODULUS_BIT_SIZE
    );
    
    let mut rng = ark_std::rand::thread_rng();
    
    log_verbose!("Generating {} random coefficients...", output_length);
    
    let a: Vec<F> = (0..output_length).map(|_| {
        let val = rng.gen_range(0..c);
        F::from(val as u64)
    }).collect();
    
    log_success!("Setup completed in {:.2?} (n={}, k={})", start_time.elapsed(), output_length, input_length);

    SecretParams {
        code: CodeParams {
            output_length,
            input_length,
            code_impl
        },
        a,
    }
}

/// Execution strategy for secret sharing operations (sequential, parallel, etc.).
pub trait ExecutionStrategy {
    fn generate_random_vec<F: PrimeField>(len: usize) -> Vec<F>;
    fn dot_product<F: PrimeField>(a: &[F], b: &[F]) -> F;
    fn create_message_matrix<F: PrimeField>(r_vec: &[F], nrows: usize, ncols: usize) -> Array2<GF2>;
    fn encode_rows(message_matrix: &Array2<GF2>, code_impl: &LdpcCode, nrows: usize, output_cols: usize) -> Array2<GF2>;
    fn decode_rows(encoded_matrix: &Array2<GF2>, code_impl: &LdpcCode, present_columns: &[bool], input_length: usize, nrows: usize) -> (Array2<GF2>, DecodingStats);
    fn reconstruct_field_elements<F: PrimeField<BigInt = BigInt<4>>>(decoded_matrix: &Array2<GF2>, input_length: usize) -> Vec<F>;
}

/// Create shares from encoded matrix columns.
pub fn create_shares_from_matrix(
    encoded_matrix: &Array2<GF2>,
    output_length: u32
) -> Vec<Share> {
    (0..output_length)
        .map(|i| Share { 
            y: encoded_matrix.column(i as usize).to_owned(), 
            i 
        })
        .collect()
}

/// Generic deal implementation using a specific execution strategy.
pub fn deal_with_strategy<F, S>(pp: &SecretParams<LdpcCode, F>, s: F) -> Shares<F>
where
    F: PrimeField,
    S: ExecutionStrategy,
{
    let start_time = Instant::now();

    // Phase 1: Random vector generation
    let rand_vec_start = Instant::now();
    let r_vec: Vec<F> = S::generate_random_vec(pp.code.input_length as usize);
    let rand_vec_duration = rand_vec_start.elapsed();

    // Phase 2: Calculate z0 = s + Σ a_i*r_i
    let dot_start = Instant::now();
    let z0 = s + S::dot_product(&pp.a, &r_vec);
    let dot_duration = dot_start.elapsed();

    // Phase 3: Message matrix creation
    let matrix_start = Instant::now();
    let nrows = <F as PrimeField>::MODULUS_BIT_SIZE as usize;
    let ncols = pp.code.input_length as usize;
    let message_matrix = S::create_message_matrix(&r_vec, nrows, ncols);
    let matrix_duration = matrix_start.elapsed();

    // Phase 4: Encoding
    let encoding_start = Instant::now();
    let output_cols = pp.code.output_length as usize;
    let encoded_matrix = S::encode_rows(
        &message_matrix, 
        &pp.code.code_impl, 
        nrows, 
        output_cols, 
    );
    let encoding_duration = encoding_start.elapsed();

    // Phase 5: Share creation
    let shares_start = Instant::now();
    let shares = create_shares_from_matrix(&encoded_matrix, pp.code.output_length);
    let shares_duration = shares_start.elapsed();

    let total_duration = start_time.elapsed();

    let metrics = DealMetrics {
        rand_vec_generation: PhaseMetrics::new("Random vector generation", rand_vec_duration, total_duration),
        dot_product: PhaseMetrics::new("Dot product calculation", dot_duration, total_duration),
        matrix_creation: PhaseMetrics::new("Message matrix creation", matrix_duration, total_duration),
        encoding: PhaseMetrics::new("Encoding phase", encoding_duration, total_duration),
        share_creation: PhaseMetrics::new("Share creation", shares_duration, total_duration),
        total_time: total_duration,
    };

    log_success!("Deal completed in {:.2?} (encoding: {:.1}%)", total_duration, metrics.encoding.percentage);
    log_verbose!("Deal breakdown: rand={:.2?} ({:.1}%), dot={:.2?} ({:.1}%), matrix={:.2?} ({:.1}%), enc={:.2?} ({:.1}%), shares={:.2?} ({:.1}%)", 
             rand_vec_duration, metrics.rand_vec_generation.percentage,
             dot_duration, metrics.dot_product.percentage,
             matrix_duration, metrics.matrix_creation.percentage,
             encoding_duration, metrics.encoding.percentage,
             shares_duration, metrics.share_creation.percentage);

    Shares {
        shares,
        z0,
        metrics: Some(metrics),
    }
}

/// Generic reconstruct implementation using a specific execution strategy.
pub fn reconstruct_with_strategy<F, S>(
    pp: &SecretParams<LdpcCode, F>, 
    shares: &Shares<F>
) -> (F, Option<ReconstructMetrics>)
where
    F: PrimeField<BigInt = BigInt<4>>,
    S: ExecutionStrategy,
{
    let start_time = Instant::now();
    let nrows = <F as PrimeField>::MODULUS_BIT_SIZE as usize;
    let ncols = pp.code.output_length as usize;

    // Build present columns mask
    let mut present_columns = vec![false; ncols];
    for share in &shares.shares {
        present_columns[share.i as usize] = true;
    }

    let missing_count = present_columns.iter().filter(|&&present| !present).count();

    // Phase 1: Matrix setup
    let setup_start = Instant::now();
    let mut encoded_matrix = Array2::<GF2>::from_elem((nrows, ncols), GF2::zero());
    for share in &shares.shares {
        encoded_matrix.column_mut(share.i as usize).assign(&share.y);
    }
    let setup_duration = setup_start.elapsed();

    // Phase 2: Decoding
    let decoding_start = Instant::now();

    let (decoded_matrix, decoding_stats) = S::decode_rows(
        &encoded_matrix,
        &pp.code.code_impl,
        &present_columns,
        pp.code.input_length as usize,
        nrows,
    );

    let decoding_duration = decoding_start.elapsed();

    // Phase 3: Field element reconstruction
    let reconstruction_start = Instant::now();

    let r: Vec<F> = S::reconstruct_field_elements(
        &decoded_matrix,
        pp.code.input_length as usize,
    );

    let reconstruction_duration = reconstruction_start.elapsed();

    // Phase 4: Final computation
    let final_start = Instant::now();
    let sum_ar = S::dot_product(&pp.a, &r);
    let result = shares.z0 - sum_ar;
    let final_duration = final_start.elapsed();

    let total_duration = start_time.elapsed();

    let metrics = ReconstructMetrics {
        matrix_setup: PhaseMetrics::new("Matrix setup", setup_duration, total_duration),
        row_decoding: PhaseMetrics::new("Row decoding", decoding_duration, total_duration),
        field_reconstruction: PhaseMetrics::new("Field element reconstruction", reconstruction_duration, total_duration),
        final_computation: PhaseMetrics::new("Final computation", final_duration, total_duration),
        total_time: total_duration,
        decoding_stats: Some(decoding_stats.clone()),
    };

    let success_rate = decoding_stats.success_rate() * 100.0;
    log_success!("Reconstruct completed in {:.2?} (decoding: {:.1}%, success: {:.1}%, avg_iter: {:.1})", 
        total_duration, metrics.row_decoding.percentage, success_rate, decoding_stats.avg_iterations);
    log_verbose!("Reconstruct: missing={}/{} ({:.1}%), decode={}/{} ok, iter_avg={:.1}, max_hit={}, setup={:.2?}, decode={:.2?}, recon={:.2?}, final={:.2?}",
             missing_count, ncols, (missing_count as f64 / ncols as f64) * 100.0,
             decoding_stats.successful_rows, nrows, decoding_stats.avg_iterations, decoding_stats.max_iterations_hit,
             setup_duration, decoding_duration, reconstruction_duration, final_duration);

    (result, Some(metrics))
}

#[cfg(test)]
mod tests {
    use super::{dot_product_adaptive, dot_product_sequential};
    use ark_bls12_381::Fr;

    #[test]
    fn test_dot_product_sequential_zeros() {
        let a: Vec<Fr> = vec![Fr::from(0u64); 5];
        let b: Vec<Fr> = vec![Fr::from(1u64); 5];
        assert_eq!(dot_product_sequential(&a, &b), Fr::from(0u64));
    }

    #[test]
    fn test_dot_product_sequential_ones() {
        let a: Vec<Fr> = vec![Fr::from(1u64); 5];
        let b: Vec<Fr> = vec![Fr::from(1u64); 5];
        assert_eq!(dot_product_sequential(&a, &b), Fr::from(5u64));
    }

    #[test]
    fn test_dot_product_sequential_mixed() {
        let a = vec![Fr::from(1u64), Fr::from(2u64), Fr::from(3u64)];
        let b = vec![Fr::from(4u64), Fr::from(5u64), Fr::from(6u64)];
        assert_eq!(dot_product_sequential(&a, &b), Fr::from(32u64));
    }

    #[test]
    fn test_dot_product_adaptive_large_vector() {
        let size = 2048;
        let a: Vec<Fr> = vec![Fr::from(2u64); size];
        let b: Vec<Fr> = vec![Fr::from(3u64); size];
        // 2 * 3 * 2048 = 12288
        assert_eq!(dot_product_adaptive(&a, &b), Fr::from(12288u64));
    }

    #[test]
    fn test_dot_product_sequential_and_adaptive_consistency() {
        let small: Vec<Fr> = (1..100).map(|i| Fr::from(i as u64)).collect();
        assert_eq!(dot_product_sequential(&small, &small), dot_product_adaptive(&small, &small));

        let large: Vec<Fr> = (1..2000).map(|i| Fr::from(i as u64)).collect();
        let large_ones: Vec<Fr> = vec![Fr::from(1u64); large.len()];
        assert_eq!(dot_product_sequential(&large, &large_ones), dot_product_adaptive(&large, &large_ones));
    }
}
