use rand::Rng;
use ark_ff::{PrimeField, BigInteger, BigInt};
use ark_std::rand::thread_rng;
use ldpc_toolbox::gf2::GF2;
use ndarray::{Array1, Array2};
use num_traits::{One, Zero};

use crate::types::{SecretParams, CodeParams, Shares, Share, CodeInitParams};
use crate::code::AdditiveCode;
use crate::code::ldpc_impl::LdpcCode;
use self::utils::{dot_product};

pub mod utils;

pub fn setup<F: PrimeField>(params: CodeInitParams, c: u32) -> SecretParams<LdpcCode, F> {
    let code_impl = LdpcCode::setup(params);
    let input_length = code_impl.input_length();
    let output_length = code_impl.output_length();
    
    assert!(input_length >= F::MODULUS_BIT_SIZE, "Number of bits ({}) must be greater than or \
    equal to the modulus bit size ({})", input_length, F::MODULUS_BIT_SIZE);
    
    let mut rng = thread_rng();
    let a: Vec<F> = (0..output_length).map(|_| {
        let val = rng.gen_range(0..c);
        F::from(val as u64)
    }).collect();

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
    let mut rng = thread_rng();

    let mut r_vec = vec![F::zero(); pp.code.input_length as usize];
    for i in 0..pp.code.input_length as usize {
        r_vec[i] = F::rand(&mut rng);
    }

    // Calculate z0 = s + Σ a_i*r_i
    let mut z0 = s;
    z0 += dot_product(&pp.a, &r_vec);

    let nrows = <F as PrimeField>::MODULUS_BIT_SIZE as usize;
    let ncols = pp.code.input_length as usize;
    let mut message_matrix = Array2::<GF2>::from_elem((nrows, ncols), GF2::zero());

    for i in 0..ncols {
        let val_int = r_vec[i].into_bigint();
        let mut bits: Vec<bool> = val_int.to_bits_le();
        bits.resize(nrows, false);
        for (j, &b) in bits.iter().enumerate() {
            message_matrix[(j, i)] = if b { GF2::one() } else { GF2::zero() };
        }
    }

    let nrows = <F as PrimeField>::MODULUS_BIT_SIZE as usize;
    let ncols = pp.code.output_length as usize;

    let mut encoded_matrix = Array2::<GF2>::from_elem((nrows, ncols), GF2::zero());
    
    for i in 0..nrows {
        let encoded = pp.code.code_impl.encode(&message_matrix.row(i).to_owned());
        encoded_matrix.row_mut(i).assign(&encoded);
    }
    
    let y: Vec<(Array1<GF2>, u32)> = (0..pp.code.output_length).map(|i| {
        let y_i = encoded_matrix.column(i as usize).to_owned();
        (y_i, i)
    }).collect();

    let shares: Vec<Share> = y.iter().map(|(y, i)| Share { y: y.clone(), i: *i }).collect();
    
    Shares { shares, z0 }
}

pub fn reconstruct<F: PrimeField<BigInt = BigInt<4>>>(pp: &mut SecretParams<LdpcCode, F>, shares: &Shares<F>) -> F {
    let nrows = <F as PrimeField>::MODULUS_BIT_SIZE as usize;
    let ncols = pp.code.output_length as usize;

    let mut present_columns = vec![false; ncols];
    for share in &shares.shares {
        present_columns[share.i as usize] = true;
    }
    
    // Count missing columns for diagnostics
    let missing_count = present_columns.iter().filter(|&&present| !present).count();
    println!("Missing columns: {} out of {} ({}%)", 
             missing_count, ncols, (missing_count as f64 / ncols as f64) * 100.0);

    let mut encoded_matrix = Array2::<GF2>::from_elem((nrows, ncols), GF2::zero());
    for share in &shares.shares {
        encoded_matrix.column_mut(share.i as usize).assign(&share.y);
    }

    let mut decoded_matrix = Array2::<GF2>::from_elem((nrows, pp.code.input_length as usize), GF2::zero());
    let mut successful_rows = 0;
    let mut failed_rows = 0;

    for i in 0..nrows {
        let row_input = encoded_matrix.row(i).to_owned();
        
        let decoded_result = pp.code.code_impl.decode(&row_input, &present_columns);

        let decoded_codeword: Vec<u8> = match decoded_result {
            Ok(decoder_output) => {
                successful_rows += 1;
                decoder_output.codeword
            },
            Err(_) => {
                failed_rows += 1;
                continue;
            }
        };

        let gf2_vec: Vec<GF2> = decoded_codeword
            .into_iter()
            .take(pp.code.input_length as usize)
            .map(|bit| if bit == 1 { GF2::one() } else { GF2::zero() })
            .collect();
        let gf2_array = Array1::from(gf2_vec);

        decoded_matrix.row_mut(i).assign(&gf2_array);
    }
    
    println!("Decoding statistics: {} rows successful, {} rows failed ({:.2}% success rate)", 
             successful_rows, failed_rows, (successful_rows as f64 / nrows as f64) * 100.0);

    let mut r = vec![F::zero(); pp.code.input_length as usize];
    for i in 0..pp.code.input_length as usize {
        let bool_vec: Vec<bool> = decoded_matrix.column(i).iter().map(|&x| x.is_one()).collect();
        let big_int = BigInteger::from_bits_le(&bool_vec);
        let val = F::from_bigint(big_int).unwrap();
        r[i] = val;
    }

    // Recover secret: s = z0 - Σ a_i*r_i
    let sum_ar = dot_product(&pp.a, &r);
    shares.z0 - sum_ar
}