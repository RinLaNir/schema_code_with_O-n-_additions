use rand::Rng;
use sparse_bin_mat::{SparseBinMat, SparseBinSlice};
use ark_ff::{Field, UniformRand, PrimeField, BigInteger};
use ark_std::rand::thread_rng;

use crate::types::{SecretParams, CodeParams, Shares, Share, CodeInitParams};
use crate::code::AdditiveCode;
use crate::code::ldpc_impl::LdpcCode;
use self::utils::{dot_product, from_slice_to_number, to_sparse_bin_vec, u32_to_field};

pub mod utils;

/// Параметризація над полем F
pub fn setup<F: PrimeField>(params: CodeInitParams, c: u32) -> SecretParams<LdpcCode, F> {
    let num_bits = params.num_bits;
    let code_impl = LdpcCode::setup(params);
    
    assert!(num_bits >= F::MODULUS_BIT_SIZE as usize, "Number of bits ({}) must be greater than or equal to the modulus bit size ({})", num_bits, F::MODULUS_BIT_SIZE);

    let k = code_impl.k();
    let mut rng = thread_rng();
    let a: Vec<F> = (0..k).map(|_| {
        // Вибираємо a[i] випадково з {0,...,c-1}, потім в поле
        let val = rng.gen_range(0..c);
        F::from(val as u64)
    }).collect();

    SecretParams {
        code: CodeParams {
            k,
            num_bits,
            code_impl
        },
        a,
    }
}

pub fn deal<F: PrimeField>(pp: &SecretParams<LdpcCode, F>, s: F) -> Shares<F> {
    let mut rng = thread_rng();

    let mut r_vec = vec![F::zero(); pp.code.k as usize];
    for i in 0..pp.code.k as usize {
        r_vec[i] = F::rand(&mut rng);
    }

    // z0 = s + Σ a_i*r_i
    let mut z0 = s;
    z0 += dot_product(&pp.a, &r_vec);

    let block_size = 32;

    let rows: Vec<Vec<usize>> = (0..pp.code.k).map(|i| {
        let val_int = &r_vec[i as usize].into_bigint();
        let mut bool_vec: Vec<bool> = val_int.to_bits_le();
        bool_vec.resize(pp.code.num_bits, false);
        bool_vec.iter().map(|&b| b as usize).collect()
    }).collect();

    let r = SparseBinMat::new(block_size, rows).transposed();
    
    let mut y = encode_slice(&r.row(0).unwrap(), &pp.code.code_impl);

    for i in 1..block_size {
        let row = r.row(i).unwrap();
        let y_i = encode_slice(&row, &pp.code.code_impl);
        y = y.vertical_concat_with(&y_i);
    }

    let y = y.transposed();
    let y: Vec<(u32, u32)> = (0..pp.code.num_bits).map(|i| {
        let y_i = from_slice_to_number(y.row(i).unwrap());
        (y_i, i as u32)
    }).collect();

    let shares: Vec<Share> = y.iter().map(|(y, i)| Share { y: *y, i: *i }).collect();

    Shares { shares, z0 }
}

pub fn reconstruct<F: PrimeField>(pp: &SecretParams<LdpcCode, F>, shares: &Shares<F>) -> F {
    let mut y_t = vec![0; pp.code.num_bits];
    for share in &shares.shares {
        y_t[share.i as usize] = share.y;
    }

    let block_size = 32;

    // Відновлюємо матрицю Y
    let y_1 = to_sparse_bin_vec(y_t[0], block_size);
    let mut y = SparseBinMat::new(block_size, vec![y_1.non_trivial_positions().collect()]);

    for i in 1..pp.code.num_bits {
        let y_i = to_sparse_bin_vec(y_t[i], block_size);
        let y_i = SparseBinMat::new(block_size, vec![y_i.non_trivial_positions().collect()]);
        y = y.vertical_concat_with(&y_i);
    }

    let y = y.transposed();

    // Декодуємо кожен рядок, щоб отримати r назад
    let y_1 = y.row(0).unwrap();
    let message = pp.code.code_impl.decode(y_1);

    let mut decoded_mat = SparseBinMat::new(pp.code.num_bits, vec![message.non_trivial_positions().collect()]);

    for i in 1..block_size {
        let y_i = y.row(i).unwrap();
        let y_i = pp.code.code_impl.decode(y_i);
        let y_i = SparseBinMat::new(pp.code.k as usize, vec![y_i.non_trivial_positions().collect()]);
        decoded_mat = decoded_mat.vertical_concat_with(&y_i);
    }

    let decoded_mat = decoded_mat.transposed();

    let mut r = vec![F::zero(); pp.code.k as usize];
    for i in 0..pp.code.k as usize {
        let val_u32 = from_slice_to_number(decoded_mat.row(i).unwrap());
        r[i] = u32_to_field(val_u32);
    }

    // s = z0 - Σ a_i*r_i
    let mut sum_ar = dot_product(&pp.a, &r);
    shares.z0 - sum_ar
}

fn encode_slice<C: AdditiveCode>(r: &SparseBinSlice, code_impl: &C) -> SparseBinMat {
    let r = SparseBinMat::new(r.len(), vec![r.non_trivial_positions().collect()]);
    code_impl.encode(&r)
}

