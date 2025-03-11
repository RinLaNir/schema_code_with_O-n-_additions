use rand::Rng;
use ark_ff::{PrimeField, BigInteger, BigInt};
use ark_std::rand::thread_rng;
use ldpc_toolbox::gf2::GF2;
use ndarray::{Array1, Array2};
use num_traits::{One, Zero};
use indicatif::{ProgressBar, ProgressStyle, MultiProgress};
use std::time::Instant;
use std::sync::{Arc, Mutex};
use rayon::prelude::*;

use crate::types::{SecretParams, CodeParams, Shares, Share, CodeInitParams,
                  PhaseMetrics, DealMetrics, ReconstructMetrics};
use crate::code::AdditiveCode;
use crate::code::ldpc_impl::LdpcCode;
use self::utils::{dot_product};

pub mod utils;

pub fn setup<F: PrimeField>(params: CodeInitParams, c: u32) -> SecretParams<LdpcCode, F> {
    let start_time = Instant::now();
    println!("Starting setup operation...");
    
    let code_impl = LdpcCode::setup(params);
    let input_length = code_impl.input_length();
    let output_length = code_impl.output_length();
    
    assert!(input_length >= F::MODULUS_BIT_SIZE, 
        "Number of bits ({}) must be greater than or equal to the modulus bit size ({})", 
        input_length, F::MODULUS_BIT_SIZE);
    
    let mut rng = thread_rng();
    
    println!("Generating {} random coefficients...", output_length);
    let progress_bar = ProgressBar::new(output_length as u64);
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} coefficients generated ({percent}%)")
            .unwrap()
            .progress_chars("##-")
    );
    
    let a: Vec<F> = (0..output_length).map(|_| {
        let val = rng.gen_range(0..c);
        progress_bar.inc(1);
        F::from(val as u64)
    }).collect();
    
    progress_bar.finish();
    println!("Setup completed in: {:.2?}", start_time.elapsed());

    SecretParams {
        code: CodeParams {
            output_length,
            input_length,
            code_impl
        },
        a,
    }
}

pub fn deal<F: PrimeField>(pp: &SecretParams<LdpcCode, F>, s: F) -> Shares<F> {
    let start_time = Instant::now();
    println!("Starting deal operation...");
    
    let mut rng = thread_rng();

    // Random vector generation phase
    let rand_vec_start = Instant::now();
    
    let progress_bar = ProgressBar::new(pp.code.input_length as u64);
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} random values generated ({percent}%)")
            .unwrap()
            .progress_chars("##-")
    );
    
    // Parallel random vector generation
    let r_vec: Vec<F> = (0..pp.code.input_length as usize)
        .into_par_iter()
        .map(|_| {
            let mut thread_rng = thread_rng();
            F::rand(&mut thread_rng)
        })
        .collect();
    
    progress_bar.set_position(pp.code.input_length as u64);
    progress_bar.finish_and_clear();
    
    let rand_vec_duration = rand_vec_start.elapsed();

    // Calculate z0 = s + Σ a_i*r_i
    let dot_start = Instant::now();
    let mut z0 = s;
    z0 += dot_product(&pp.a, &r_vec);
    let dot_duration = dot_start.elapsed();

    // Message matrix creation
    let matrix_start = Instant::now();
    let nrows = <F as PrimeField>::MODULUS_BIT_SIZE as usize;
    let ncols = pp.code.input_length as usize;
    let mut message_matrix = Array2::<GF2>::from_elem((nrows, ncols), GF2::zero());

    let matrix_progress = ProgressBar::new(ncols as u64);
    matrix_progress.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.yellow/blue} {pos}/{len} columns processed ({percent}%)")
            .unwrap()
            .progress_chars("##-")
    );
    
    let message_matrix = Arc::new(Mutex::new(message_matrix));
    (0..ncols).into_par_iter().for_each(|i| {
        let val_int = r_vec[i].into_bigint();
        let mut bits: Vec<bool> = val_int.to_bits_le();
        bits.resize(nrows, false);
        
        let column_data: Vec<GF2> = bits.iter()
            .map(|&b| if b { GF2::one() } else { GF2::zero() })
            .collect();
        
        // Update the shared matrix with a single lock
        let mut matrix = message_matrix.lock().unwrap();
        for (j, &value) in column_data.iter().enumerate() {
            matrix[(j, i)] = value;
        }
        
        matrix_progress.inc(1);
    });
    matrix_progress.finish_and_clear();
    
    let matrix_duration = matrix_start.elapsed();
    let message_matrix = Arc::try_unwrap(message_matrix)
        .expect("Failed to unwrap Arc")
        .into_inner()
        .expect("Failed to unwrap Mutex");

    let nrows = <F as PrimeField>::MODULUS_BIT_SIZE as usize;
    let ncols = pp.code.output_length as usize;

    let encoding_start = Instant::now();
    let mut encoded_matrix = Array2::<GF2>::from_elem((nrows, ncols), GF2::zero());
    
    let encoding_progress = ProgressBar::new(nrows as u64);
    encoding_progress.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.green/blue} {pos}/{len} rows encoded ({percent}%)")
            .unwrap()
            .progress_chars("##-")
    );
    
    // Parallel row encoding with shared result matrix
    let encoded_matrix = Arc::new(Mutex::new(encoded_matrix));
    (0..nrows).into_par_iter().for_each(|i| {
        let row = message_matrix.row(i).to_owned();
        let encoded = pp.code.code_impl.encode(&row);
        
        let mut matrix = encoded_matrix.lock().unwrap();
        matrix.row_mut(i).assign(&encoded);
        
        encoding_progress.inc(1);
    });
    
    encoding_progress.finish_with_message("encoding completed");
    
    let encoding_duration = encoding_start.elapsed();
    let encoded_matrix = Arc::try_unwrap(encoded_matrix)
        .expect("Failed to unwrap Arc")
        .into_inner()
        .expect("Failed to unwrap Mutex");
    
    let shares_start = Instant::now();
    let y: Vec<(Array1<GF2>, u32)> = (0..pp.code.output_length)
        .into_par_iter()
        .map(|i| {
            let y_i = encoded_matrix.column(i as usize).to_owned();
            (y_i, i)
        })
        .collect();

    let shares: Vec<Share> = y.iter().map(|(y, i)| Share { y: y.clone(), i: *i }).collect();
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
    
    // Print metrics for debugging during development
    println!("Deal operation performance breakdown:");
    println!("  - Random vector generation: {:.2?} ({:.2}%)", 
             rand_vec_duration, metrics.rand_vec_generation.percentage);
    println!("  - Dot product calculation: {:.2?} ({:.2}%)", 
             dot_duration, metrics.dot_product.percentage);
    println!("  - Message matrix creation: {:.2?} ({:.2}%)", 
             matrix_duration, metrics.matrix_creation.percentage);
    println!("  - Encoding phase: {:.2?} ({:.2}%)", 
             encoding_duration, metrics.encoding.percentage);
    println!("  - Share creation: {:.2?} ({:.2}%)", 
             shares_duration, metrics.share_creation.percentage);
    println!("  - Total deal time: {:.2?}", total_duration);
    
    Shares {
        shares, 
        z0,
        metrics: Some(metrics),
    }
}

pub fn reconstruct<F: PrimeField<BigInt = BigInt<4>>>(pp: &SecretParams<LdpcCode, F>, shares: &Shares<F>) -> (F, Option<ReconstructMetrics>) {
    let start_time = Instant::now();
    let nrows = <F as PrimeField>::MODULUS_BIT_SIZE as usize;
    let ncols = pp.code.output_length as usize;

    let mut present_columns = vec![false; ncols];
    for share in &shares.shares {
        present_columns[share.i as usize] = true;
    }
    
    let missing_count = present_columns.iter().filter(|&&present| !present).count();
    println!("Missing columns: {} out of {} ({}%)", 
             missing_count, ncols, (missing_count as f64 / ncols as f64) * 100.0);

    let setup_start = Instant::now();
    let mut encoded_matrix = Array2::<GF2>::from_elem((nrows, ncols), GF2::zero());
    for share in &shares.shares {
        encoded_matrix.column_mut(share.i as usize).assign(&share.y);
    }
    let setup_duration = setup_start.elapsed();

    let decoded_matrix = Arc::new(Mutex::new(
        Array2::<GF2>::from_elem((nrows, pp.code.input_length as usize), GF2::zero())
    ));
    let successful_rows = Arc::new(Mutex::new(0));
    let failed_rows = Arc::new(Mutex::new(0));

    let progress_bar = ProgressBar::new(nrows as u64);
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} rows decoded ({percent}%) - {msg}")
            .unwrap()
            .progress_chars("##-")
    );
    progress_bar.set_message("decoding in progress...");
    progress_bar.enable_steady_tick(std::time::Duration::from_millis(200));

    let decoding_start = Instant::now();
    
    // Parallel decoding
    let present_columns = Arc::new(present_columns);
    let encoded_matrix = Arc::new(encoded_matrix);
    
    (0..nrows).into_par_iter().for_each(|i| {
        let row_input = encoded_matrix.row(i).to_owned();
        
        let decoded_result = pp.code.code_impl.decode(&row_input, &present_columns);

        match decoded_result {
            Ok(decoder_output) => {
                let gf2_vec: Vec<GF2> = decoder_output.codeword
                    .into_iter()
                    .take(pp.code.input_length as usize)
                    .map(|bit| if bit == 1 { GF2::one() } else { GF2::zero() })
                    .collect();
                
                let gf2_array = Array1::from(gf2_vec);
                
                let mut matrix = decoded_matrix.lock().unwrap();
                matrix.row_mut(i).assign(&gf2_array);
                
                let mut successful = successful_rows.lock().unwrap();
                *successful += 1;
            },
            Err(_) => {
                let mut failed = failed_rows.lock().unwrap();
                *failed += 1;
            }
        }
        
        progress_bar.inc(1);
    });
    
    let successful_count = *successful_rows.lock().unwrap();
    let failed_count = *failed_rows.lock().unwrap();
    
    let decoding_duration = decoding_start.elapsed();
    progress_bar.finish_with_message(format!(
        "decoding completed in {:.2?}: {:.2}% success rate", 
        decoding_duration,
        (successful_count as f64 / nrows as f64) * 100.0
    ));
    
    println!("Decoding statistics: {} rows successful, {} rows failed ({:.2}% success rate)", 
             successful_count, failed_count, (successful_count as f64 / nrows as f64) * 100.0);

    let reconstruction_start = Instant::now();
    let reconstruct_bar = ProgressBar::new(pp.code.input_length as u64);
    reconstruct_bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.green/blue} {pos}/{len} values reconstructed ({percent}%)")
            .unwrap()
            .progress_chars("##-")
    );

    let decoded_matrix = Arc::try_unwrap(decoded_matrix)
        .expect("Failed to unwrap Arc")
        .into_inner()
        .expect("Failed to unwrap Mutex");
    
    // Reconstruct field elements in parallel
    let r: Vec<F> = (0..pp.code.input_length as usize)
        .into_par_iter()
        .map(|i| {
            let bool_vec: Vec<bool> = decoded_matrix.column(i).iter().map(|&x| x.is_one()).collect();
            let big_int = BigInteger::from_bits_le(&bool_vec);
            let val = F::from_bigint(big_int).unwrap();
            reconstruct_bar.inc(1);
            val
        })
        .collect();
    
    let reconstruction_duration = reconstruction_start.elapsed();
    reconstruct_bar.finish_with_message(format!(
        "field elements reconstructed in {:.2?}", 
        reconstruction_duration
    ));

    let final_start = Instant::now();
    let sum_ar = dot_product(&pp.a, &r);
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
    };
    
    println!("Reconstruction performance breakdown:");
    println!("  - Matrix setup: {:.2?} ({:.2}%)", 
             setup_duration, metrics.matrix_setup.percentage);
    println!("  - Row decoding: {:.2?} ({:.2}%)", 
             decoding_duration, metrics.row_decoding.percentage);
    println!("  - Field element reconstruction: {:.2?} ({:.2}%)", 
             reconstruction_duration, metrics.field_reconstruction.percentage);
    println!("  - Final computation: {:.2?} ({:.2}%)", 
             final_duration, metrics.final_computation.percentage);
    println!("  - Total reconstruction time: {:.2?}", total_duration);
    
    (result, Some(metrics))
}