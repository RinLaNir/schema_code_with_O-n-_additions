use ark_bls12_381::Fr;
use ark_ff::{PrimeField};
use rand::seq::SliceRandom;
use rand::thread_rng;

mod types;
mod code;
mod aos;

use aos::{setup, deal, reconstruct};
use types::CodeInitParams;
use crate::types::Share;

fn main() {
    let secret = Fr::from(42u128); // Secret as a field element
    let c = 10;
    let code_params = CodeInitParams {
        num_bits: 16,
        num_checks: 12,
        bit_degree: 3,
        check_degree: 4,
    };

    let mut pp = setup::<Fr>(code_params, c);
    let mut shares = deal(&pp, secret);
    
    println!("Total shares: {}", shares.shares.len());
    
    let shares_to_remove = 100;
    println!("Removing {} random shares...", shares_to_remove);
    remove_random_shares(&mut shares.shares, shares_to_remove);
    println!("Remaining shares: {}", shares.shares.len());

    let reconstructed_secret = reconstruct(&mut pp, &shares);

    println!("Original Secret: {:?}", secret.into_bigint());
    println!("Reconstructed Secret: {:?}", reconstructed_secret.into_bigint());
    
    if secret == reconstructed_secret {
        println!("✅ Secret reconstructed successfully!");
    } else {
        println!("❌ Secret reconstruction failed!");
    }
}

/// Removes random shares from the vector
fn remove_random_shares(shares: &mut Vec<Share>, num_to_remove: usize) {
    let mut rng = thread_rng();
    shares.shuffle(&mut rng);
    shares.drain(0..num_to_remove);
}