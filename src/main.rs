use ark_bls12_381::Fr;
use ark_ff::{PrimeField};
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::time::Instant;
use chrono::Local;

mod types;
mod code;
mod aos;

use aos::{setup, deal, reconstruct};
use types::CodeInitParams;
use crate::types::Share;

fn main() {
    println!("Starting secret sharing scheme at: {}", Local::now().format("%Y-%m-%d %H:%M:%S"));
    
    let secret = Fr::from(42u128); // Secret as a field element
    let c = 10;
    let code_params = CodeInitParams {
        num_bits: 16,
        num_checks: 12,
        bit_degree: 3,
        check_degree: 4,
    };

    let setup_start = Instant::now();
    let mut pp = setup::<Fr>(code_params, c);
    let setup_duration = setup_start.elapsed();
    
    let deal_start = Instant::now();
    let mut shares = deal(&pp, secret);
    let deal_duration = deal_start.elapsed();
    
    println!("Total shares: {}", shares.shares.len());
    
    let shares_to_remove = 100;
    println!("Removing {} random shares...", shares_to_remove);
    remove_random_shares(&mut shares.shares, shares_to_remove);
    println!("Remaining shares: {}", shares.shares.len());

    // Measure reconstruction time
    let reconstruct_start = Instant::now();
    let reconstructed_secret = reconstruct(&mut pp, &shares);
    let reconstruct_duration = reconstruct_start.elapsed();

    println!("Original Secret: {:?}", secret.into_bigint());
    println!("Reconstructed Secret: {:?}", reconstructed_secret.into_bigint());
    
    if secret == reconstructed_secret {
        println!("✅ Secret reconstructed successfully!");
    } else {
        println!("❌ Secret reconstruction failed!");
    }
    
    // Print overall performance summary
    let total_time = setup_duration + deal_duration + reconstruct_duration;
    println!("\n--- Performance Summary ---");
    println!("Setup: {:.2?} ({:.2}%)", 
             setup_duration, (setup_duration.as_secs_f64() / total_time.as_secs_f64()) * 100.0);
    println!("Deal: {:.2?} ({:.2}%)", 
             deal_duration, (deal_duration.as_secs_f64() / total_time.as_secs_f64()) * 100.0);
    println!("Reconstruction: {:.2?} ({:.2}%)", 
             reconstruct_duration, (reconstruct_duration.as_secs_f64() / total_time.as_secs_f64()) * 100.0);
    println!("Total execution time: {:.2?}", total_time);
}

/// Removes random shares from the vector
fn remove_random_shares(shares: &mut Vec<Share>, num_to_remove: usize) {
    let mut rng = thread_rng();
    shares.shuffle(&mut rng);
    shares.drain(0..num_to_remove);
}