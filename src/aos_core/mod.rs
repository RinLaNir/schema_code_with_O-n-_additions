//! Core module for secret sharing operations using LDPC codes.
//! 
//! This module contains shared logic for both sequential and parallel implementations.
//! It provides the `setup()` function and the `ExecutionStrategy` trait that defines
//! how different phases of deal/reconstruct operations are executed.

use rand::Rng;
use ark_ff::{PrimeField, BigInt};
use ark_std::rand::thread_rng;
use ldpc_toolbox::gf2::GF2;
use ndarray::Array2;
use num_traits::Zero;
use indicatif::ProgressBar;
use std::time::Instant;

use crate::types::{
    SecretParams, CodeParams, Shares, Share, CodeInitParams,
    PhaseMetrics, DealMetrics, ReconstructMetrics, DecodingStats
};
use crate::code::AdditiveCode;
use crate::code::ldpc_impl::LdpcCode;
use crate::{log_verbose, log_success};

/// Creates a new progress bar with a consistent style.
/// Progress bars are hidden to avoid terminal output - progress is tracked internally.
pub fn create_progress_bar(total: u64, _template: &str) -> ProgressBar {
    let pb = ProgressBar::new(total);
    // Hide progress bar output - we only use it for internal tracking
    pb.set_draw_target(indicatif::ProgressDrawTarget::hidden());
    pb
}

/// Progress bar templates for different operations
pub mod progress_templates {
    pub const COEFFICIENTS: &str = "[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} coefficients generated ({percent}%)";
    pub const RANDOM_VALUES: &str = "[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} random values generated ({percent}%)";
    pub const COLUMNS: &str = "[{elapsed_precise}] {bar:40.yellow/blue} {pos}/{len} columns processed ({percent}%)";
    pub const ENCODING: &str = "[{elapsed_precise}] {bar:40.green/blue} {pos}/{len} rows encoded ({percent}%)";
    pub const DECODING: &str = "[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} rows decoded ({percent}%) - {msg}";
    pub const RECONSTRUCTION: &str = "[{elapsed_precise}] {bar:40.green/blue} {pos}/{len} values reconstructed ({percent}%)";
}

/// Setup the secret sharing scheme parameters.
/// 
/// This function initializes the LDPC code and generates random coefficients.
/// The implementation is identical for both sequential and parallel versions.
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
    
    let mut rng = thread_rng();
    
    log_verbose!("Generating {} random coefficients...", output_length);
    let progress_bar = create_progress_bar(output_length as u64, progress_templates::COEFFICIENTS);
    
    let a: Vec<F> = (0..output_length).map(|_| {
        let val = rng.gen_range(0..c);
        progress_bar.inc(1);
        F::from(val as u64)
    }).collect();
    
    progress_bar.finish();
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

/// Trait defining execution strategy for secret sharing operations.
/// 
/// This trait allows different implementations (sequential, parallel, GPU, etc.)
/// to provide their own execution strategy for computationally intensive phases.
pub trait ExecutionStrategy {
    /// Generate a random vector of field elements.
    fn generate_random_vec<F: PrimeField>(len: usize) -> Vec<F>;
    
    /// Compute dot product of two vectors.
    fn dot_product<F: PrimeField>(a: &[F], b: &[F]) -> F;
    
    /// Create message matrix from random vector.
    /// Converts field elements to GF2 bit representation.
    fn create_message_matrix<F: PrimeField>(
        r_vec: &[F], 
        nrows: usize, 
        ncols: usize,
        progress_bar: &ProgressBar
    ) -> Array2<GF2>;
    
    /// Encode rows of the message matrix using the LDPC code.
    fn encode_rows(
        message_matrix: &Array2<GF2>,
        code_impl: &LdpcCode,
        nrows: usize,
        output_cols: usize,
        progress_bar: &ProgressBar
    ) -> Array2<GF2>;
    
    /// Decode rows using the LDPC decoder.
    /// Returns decoded matrix and DecodingStats (iterations, success counts, etc.).
    fn decode_rows(
        encoded_matrix: &Array2<GF2>,
        code_impl: &LdpcCode,
        present_columns: &[bool],
        input_length: usize,
        nrows: usize,
        progress_bar: &ProgressBar
    ) -> (Array2<GF2>, DecodingStats);
    
    /// Reconstruct field elements from decoded GF2 matrix.
    fn reconstruct_field_elements<F: PrimeField<BigInt = BigInt<4>>>(
        decoded_matrix: &Array2<GF2>,
        input_length: usize,
        progress_bar: &ProgressBar
    ) -> Vec<F>;
}

/// Create shares from encoded matrix.
/// This is common logic for both sequential and parallel implementations.
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
    let progress_bar = create_progress_bar(
        pp.code.input_length as u64, 
        progress_templates::RANDOM_VALUES
    );
    let r_vec: Vec<F> = S::generate_random_vec(pp.code.input_length as usize);
    progress_bar.set_position(pp.code.input_length as u64);
    progress_bar.finish_and_clear();
    let rand_vec_duration = rand_vec_start.elapsed();

    // Phase 2: Calculate z0 = s + Σ a_i*r_i
    let dot_start = Instant::now();
    let mut z0 = s;
    z0 += S::dot_product(&pp.a, &r_vec);
    let dot_duration = dot_start.elapsed();

    // Phase 3: Message matrix creation
    let matrix_start = Instant::now();
    let nrows = <F as PrimeField>::MODULUS_BIT_SIZE as usize;
    let ncols = pp.code.input_length as usize;
    let matrix_progress = create_progress_bar(ncols as u64, progress_templates::COLUMNS);
    let message_matrix = S::create_message_matrix(&r_vec, nrows, ncols, &matrix_progress);
    matrix_progress.finish_and_clear();
    let matrix_duration = matrix_start.elapsed();

    // Phase 4: Encoding
    let encoding_start = Instant::now();
    let output_cols = pp.code.output_length as usize;
    let encoding_progress = create_progress_bar(nrows as u64, progress_templates::ENCODING);
    let encoded_matrix = S::encode_rows(
        &message_matrix, 
        &pp.code.code_impl, 
        nrows, 
        output_cols, 
        &encoding_progress
    );
    encoding_progress.finish_with_message("encoding completed");
    let encoding_duration = encoding_start.elapsed();

    // Phase 5: Share creation
    let shares_start = Instant::now();
    let shares = create_shares_from_matrix(&encoded_matrix, pp.code.output_length);
    let shares_duration = shares_start.elapsed();

    let total_duration = start_time.elapsed();

    // Create metrics
    let metrics = DealMetrics {
        rand_vec_generation: PhaseMetrics::new("Random vector generation", rand_vec_duration, total_duration),
        dot_product: PhaseMetrics::new("Dot product calculation", dot_duration, total_duration),
        matrix_creation: PhaseMetrics::new("Message matrix creation", matrix_duration, total_duration),
        encoding: PhaseMetrics::new("Encoding phase", encoding_duration, total_duration),
        share_creation: PhaseMetrics::new("Share creation", shares_duration, total_duration),
        total_time: total_duration,
    };

    // Summary log (always shown)
    log_success!("Deal completed in {:.2?} (encoding: {:.1}%)", total_duration, metrics.encoding.percentage);
    
    // Verbose breakdown (only when verbose mode enabled)
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
    let progress_bar = create_progress_bar(nrows as u64, progress_templates::DECODING);
    progress_bar.set_message("decoding in progress...");

    let (decoded_matrix, decoding_stats) = S::decode_rows(
        &encoded_matrix,
        &pp.code.code_impl,
        &present_columns,
        pp.code.input_length as usize,
        nrows,
        &progress_bar
    );

    let decoding_duration = decoding_start.elapsed();
    progress_bar.finish_with_message(format!(
        "decoding completed in {:.2?}: {:.2}% success rate",
        decoding_duration,
        decoding_stats.success_rate() * 100.0
    ));

    // Phase 3: Field element reconstruction
    let reconstruction_start = Instant::now();
    let reconstruct_bar = create_progress_bar(
        pp.code.input_length as u64, 
        progress_templates::RECONSTRUCTION
    );

    let r: Vec<F> = S::reconstruct_field_elements(
        &decoded_matrix,
        pp.code.input_length as usize,
        &reconstruct_bar
    );

    let reconstruction_duration = reconstruction_start.elapsed();
    reconstruct_bar.finish_with_message(format!(
        "field elements reconstructed in {:.2?}",
        reconstruction_duration
    ));

    // Phase 4: Final computation
    let final_start = Instant::now();
    let sum_ar = S::dot_product(&pp.a, &r);
    let result = shares.z0 - sum_ar;
    let final_duration = final_start.elapsed();

    let total_duration = start_time.elapsed();

    // Create metrics
    let metrics = ReconstructMetrics {
        matrix_setup: PhaseMetrics::new("Matrix setup", setup_duration, total_duration),
        row_decoding: PhaseMetrics::new("Row decoding", decoding_duration, total_duration),
        field_reconstruction: PhaseMetrics::new("Field element reconstruction", reconstruction_duration, total_duration),
        final_computation: PhaseMetrics::new("Final computation", final_duration, total_duration),
        total_time: total_duration,
        decoding_stats: Some(decoding_stats.clone()),
    };

    // Summary log (always shown)
    let success_rate = decoding_stats.success_rate() * 100.0;
    log_success!("Reconstruct completed in {:.2?} (decoding: {:.1}%, success: {:.1}%, avg_iter: {:.1})", 
        total_duration, metrics.row_decoding.percentage, success_rate, decoding_stats.avg_iterations);
    
    // Verbose breakdown (only when verbose mode enabled)
    log_verbose!("Reconstruct: missing={}/{} ({:.1}%), decode={}/{} ok, iter_avg={:.1}, max_hit={}, setup={:.2?}, decode={:.2?}, recon={:.2?}, final={:.2?}",
             missing_count, ncols, (missing_count as f64 / ncols as f64) * 100.0,
             decoding_stats.successful_rows, nrows, decoding_stats.avg_iterations, decoding_stats.max_iterations_hit,
             setup_duration, decoding_duration, reconstruction_duration, final_duration);

    (result, Some(metrics))
}
