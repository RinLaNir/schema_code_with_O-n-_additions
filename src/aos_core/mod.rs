//! Core module for secret sharing operations using LDPC codes.

use ldpc_toolbox::gf2::GF2;
use ndarray::Array2;
use num_traits::{One, Zero};
use std::time::Instant;

use crate::code::ldpc_impl::LdpcCode;
use crate::code::AdditiveCode;
use crate::types::{
    CodeInitParams, CodeParams, DealMetrics, DecodingStats, F2PowElement, PhaseMetrics,
    ReconstructMetrics, SecretParams, Share, Shares,
};
use crate::{log_success, log_verbose, log_warning};

#[inline]
pub fn codeword_to_gf2_buf(codeword: &[u8], out: &mut [GF2], len: usize) {
    let gf2_one = GF2::one();
    let gf2_zero = GF2::zero();
    for (dst, &byte) in out[..len].iter_mut().zip(codeword.iter().take(len)) {
        *dst = if byte == 1 { gf2_one } else { gf2_zero };
    }
}

#[inline]
pub fn masked_xor(
    secret: &F2PowElement,
    a_bits: &[bool],
    columns: &[F2PowElement],
) -> F2PowElement {
    assert_eq!(
        a_bits.len(),
        columns.len(),
        "mask length and column count must match"
    );

    let mut acc = secret.clone();
    for (a_bit, column) in a_bits.iter().zip(columns) {
        if *a_bit {
            acc.xor_assign(column);
        }
    }
    acc
}

pub fn setup(params: CodeInitParams) -> SecretParams<LdpcCode> {
    let start_time = Instant::now();
    log_verbose!("Starting setup operation...");

    let ell = params.secret_bits.unwrap_or(128);
    let code_impl = LdpcCode::setup(params);
    let input_length = code_impl.input_length();
    let output_length = code_impl.output_length();

    assert!(
        input_length >= ell as u32,
        "Information length ({}) must be >= secret bits ({})",
        input_length,
        ell
    );

    let mut rng = rand::rng();
    let a_mask = F2PowElement::random(input_length as usize, &mut rng);
    let a_bits: Vec<bool> = (0..input_length as usize).map(|i| a_mask.bit(i)).collect();

    log_success!(
        "Setup completed in {:.2?} (n={}, k={}, ell={})",
        start_time.elapsed(),
        output_length,
        input_length,
        ell
    );

    SecretParams {
        code: CodeParams {
            output_length,
            input_length,
            code_impl,
        },
        ell,
        a_bits,
    }
}

pub trait ExecutionStrategy {
    fn generate_random_columns(len: usize, bit_len: usize) -> Vec<F2PowElement>;
    fn create_message_matrix(columns: &[F2PowElement], nrows: usize, ncols: usize) -> Array2<GF2>;
    fn encode_rows(
        message_matrix: &Array2<GF2>,
        code_impl: &LdpcCode,
        nrows: usize,
        output_cols: usize,
    ) -> Array2<GF2>;
    fn decode_rows(
        encoded_matrix: &Array2<GF2>,
        code_impl: &LdpcCode,
        present_columns: &[bool],
        input_length: usize,
        nrows: usize,
    ) -> (Array2<GF2>, DecodingStats);
    fn reconstruct_columns(
        decoded_matrix: &Array2<GF2>,
        input_length: usize,
        bit_len: usize,
    ) -> Vec<F2PowElement>;
}

pub fn create_shares_from_matrix(encoded_matrix: &Array2<GF2>, output_length: u32) -> Vec<Share> {
    (0..output_length)
        .map(|i| Share {
            y: encoded_matrix.column(i as usize).to_owned(),
            i,
        })
        .collect()
}

pub fn deal_with_strategy<S>(pp: &SecretParams<LdpcCode>, secret: &F2PowElement) -> Shares
where
    S: ExecutionStrategy,
{
    assert_eq!(
        secret.bit_len, pp.ell,
        "secret bit length must match setup ell"
    );

    let start_time = Instant::now();

    let rand_vec_start = Instant::now();
    let r_vec = S::generate_random_columns(pp.code.input_length as usize, pp.ell);
    let rand_vec_duration = rand_vec_start.elapsed();

    let mask_start = Instant::now();
    let z0 = masked_xor(secret, &pp.a_bits, &r_vec);
    let mask_duration = mask_start.elapsed();

    let matrix_start = Instant::now();
    let nrows = pp.ell;
    let ncols = pp.code.input_length as usize;
    let message_matrix = S::create_message_matrix(&r_vec, nrows, ncols);
    let matrix_duration = matrix_start.elapsed();

    let encoding_start = Instant::now();
    let output_cols = pp.code.output_length as usize;
    let encoded_matrix = S::encode_rows(&message_matrix, &pp.code.code_impl, nrows, output_cols);
    let encoding_duration = encoding_start.elapsed();

    let shares_start = Instant::now();
    let shares = create_shares_from_matrix(&encoded_matrix, pp.code.output_length);
    let shares_duration = shares_start.elapsed();

    let total_duration = start_time.elapsed();

    let metrics = DealMetrics {
        rand_vec_generation: PhaseMetrics::new(
            "Random vector generation",
            rand_vec_duration,
            total_duration,
        ),
        mask_xor: PhaseMetrics::new("Mask XOR", mask_duration, total_duration),
        matrix_creation: PhaseMetrics::new(
            "Message matrix creation",
            matrix_duration,
            total_duration,
        ),
        encoding: PhaseMetrics::new("Encoding phase", encoding_duration, total_duration),
        share_creation: PhaseMetrics::new("Share creation", shares_duration, total_duration),
        total_time: total_duration,
    };

    log_success!(
        "Deal completed in {:.2?} (encoding: {:.1}%)",
        total_duration,
        metrics.encoding.percentage
    );
    log_verbose!(
        "Deal breakdown: rand={:.2?} ({:.1}%), mask={:.2?} ({:.1}%), matrix={:.2?} ({:.1}%), enc={:.2?} ({:.1}%), shares={:.2?} ({:.1}%)",
        rand_vec_duration,
        metrics.rand_vec_generation.percentage,
        mask_duration,
        metrics.mask_xor.percentage,
        matrix_duration,
        metrics.matrix_creation.percentage,
        encoding_duration,
        metrics.encoding.percentage,
        shares_duration,
        metrics.share_creation.percentage
    );

    Shares {
        shares,
        z0,
        metrics: Some(metrics),
    }
}

pub fn reconstruct_with_strategy<S>(
    pp: &SecretParams<LdpcCode>,
    shares: &Shares,
) -> (Option<F2PowElement>, Option<ReconstructMetrics>)
where
    S: ExecutionStrategy,
{
    let start_time = Instant::now();
    let nrows = pp.ell;
    let ncols = pp.code.output_length as usize;

    let mut present_columns = vec![false; ncols];
    for share in &shares.shares {
        present_columns[share.i as usize] = true;
    }
    let missing_count = present_columns.iter().filter(|&&present| !present).count();

    let setup_start = Instant::now();
    let mut encoded_matrix = Array2::<GF2>::from_elem((nrows, ncols), GF2::zero());
    for share in &shares.shares {
        encoded_matrix.column_mut(share.i as usize).assign(&share.y);
    }
    let setup_duration = setup_start.elapsed();

    let decoding_start = Instant::now();
    let (decoded_matrix, decoding_stats) = S::decode_rows(
        &encoded_matrix,
        &pp.code.code_impl,
        &present_columns,
        pp.code.input_length as usize,
        nrows,
    );
    let decoding_duration = decoding_start.elapsed();

    let (result, reconstruction_duration, final_duration) = if decoding_stats.failed_rows == 0 {
        let reconstruction_start = Instant::now();
        let r = S::reconstruct_columns(&decoded_matrix, pp.code.input_length as usize, pp.ell);
        let reconstruction_duration = reconstruction_start.elapsed();

        let final_start = Instant::now();
        let result = masked_xor(&shares.z0, &pp.a_bits, &r);
        let final_duration = final_start.elapsed();

        (Some(result), reconstruction_duration, final_duration)
    } else {
        (None, std::time::Duration::ZERO, std::time::Duration::ZERO)
    };

    let total_duration = start_time.elapsed();

    let metrics = ReconstructMetrics {
        matrix_setup: PhaseMetrics::new("Matrix setup", setup_duration, total_duration),
        row_decoding: PhaseMetrics::new("Row decoding", decoding_duration, total_duration),
        column_reconstruction: PhaseMetrics::new(
            "Column reconstruction",
            reconstruction_duration,
            total_duration,
        ),
        final_computation: PhaseMetrics::new("Final computation", final_duration, total_duration),
        total_time: total_duration,
        decoding_stats: Some(decoding_stats.clone()),
    };

    let success_rate = decoding_stats.success_rate() * 100.0;
    if result.is_some() {
        log_success!(
            "Reconstruct completed in {:.2?} (decoding: {:.1}%, success: {:.1}%, avg_iter: {:.1})",
            total_duration,
            metrics.row_decoding.percentage,
            success_rate,
            decoding_stats.avg_iterations
        );
    } else {
        log_warning!(
            "Reconstruct failed in {:.2?} (decoded rows: {}/{}, avg_iter: {:.1})",
            total_duration,
            decoding_stats.successful_rows,
            decoding_stats.total_rows,
            decoding_stats.avg_iterations
        );
    }
    log_verbose!(
        "Reconstruct: missing={}/{} ({:.1}%), decode={}/{} ok, iter_avg={:.1}, max_hit={}, setup={:.2?}, decode={:.2?}, cols={:.2?}, final={:.2?}",
        missing_count,
        ncols,
        (missing_count as f64 / ncols as f64) * 100.0,
        decoding_stats.successful_rows,
        nrows,
        decoding_stats.avg_iterations,
        decoding_stats.max_iterations_hit,
        setup_duration,
        decoding_duration,
        reconstruction_duration,
        final_duration
    );

    (result, Some(metrics))
}

#[cfg(test)]
mod tests {
    use super::masked_xor;
    use crate::types::F2PowElement;

    #[test]
    fn test_masked_xor_applies_selected_columns() {
        let secret = F2PowElement::from_hex("0003", 16).unwrap();
        let cols = vec![
            F2PowElement::from_hex("0001", 16).unwrap(),
            F2PowElement::from_hex("0002", 16).unwrap(),
            F2PowElement::from_hex("0004", 16).unwrap(),
        ];
        let result = masked_xor(&secret, &[true, false, true], &cols);
        assert_eq!(result.to_hex(), "0006");
    }
}
