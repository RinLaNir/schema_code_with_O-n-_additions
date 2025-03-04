use ark_bls12_381::Fr;
use ark_ff::{Field, PrimeField};
use rand::seq::SliceRandom;
use rand::thread_rng;

mod types;
mod code;
mod aos;

use aos::{setup, deal, reconstruct};
use types::CodeInitParams;
use crate::types::Share;

fn main() {
    let secret = Fr::from(42u128); // Секрет як елемент поля
    let c = 10;
    let code_params = CodeInitParams {
        num_bits: 16,
        num_checks: 12,
        bit_degree: 3,
        check_degree: 4,
    };

    let mut pp = setup::<Fr>(code_params, c);
    let mut shares = deal(&pp, secret);
    
    remove_random_shares(&mut shares.shares, 1);

    let reconstructed_secret = reconstruct(&mut pp, &shares);

    println!("Original Secret: {:?}", secret.into_bigint());
    println!("Reconstructed Secret: {:?}", reconstructed_secret.into_bigint());
}

fn remove_random_shares(shares: &mut Vec<Share>, num_to_remove: usize) {
    let mut rng = thread_rng();
    // Перемішуємо вектор
    shares.shuffle(&mut rng);
    // Видаляємо перші num_to_remove елементів
    shares.drain(0..num_to_remove);
}