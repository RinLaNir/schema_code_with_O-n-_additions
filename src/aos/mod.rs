use rand::{Rng, random};
use sparse_bin_mat::{SparseBinMat, SparseBinSlice, SparseBinVec};

use crate::types::{SecretParams, CodeParams, Shares, Share, CodeInitParams};
use crate::code::AdditiveCode;
use crate::code::ldpc_impl::LdpcCode;
use self::utils::{dot_product, from_slice_to_number, to_sparse_bin_vec};

pub mod utils;

pub fn setup(params: CodeInitParams, c: u32) -> SecretParams<LdpcCode> {
    let num_bits = params.num_bits;
    let code_impl = LdpcCode::setup(params);

    let k = code_impl.k();
    let a: Vec<u32> = (0..k).map(|_| rand::thread_rng().gen_range(0..c)).collect();

    SecretParams {
        code: CodeParams {
            k,
            num_bits,
            code_impl,
        },
        a,
    }
}

pub fn deal<C: AdditiveCode>(pp: &SecretParams<C>, s: u32) -> Shares {
    let mut r_vec = vec![0; pp.code.k as usize];
    for i in 0..pp.code.k as usize {
        r_vec[i] = random();
    }

    // Обчислення z0
    let z0 = s as u64 + dot_product(&pp.a, &r_vec);

    let block_size = 32;

    // Створюємо блокову матрицю з r_vec
    let rows: Vec<Vec<usize>> = (0..pp.code.k).map(|i| {
        let mut row = Vec::new();
        for j in 0..block_size {
            if (r_vec[i as usize] >> j) & 1 == 1 {
                row.push(j);
            }
        }
        row
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

pub fn reconstruct<C: AdditiveCode>(pp: &SecretParams<C>, shares: &Shares) -> u32 {
    let mut y_t = vec![0; pp.code.num_bits];
    for share in &shares.shares {
        y_t[share.i as usize] = share.y;
    }

    let block_size = 32;

    // Відновлюємо матрицю Y з бітів
    let y_1 = to_sparse_bin_vec(y_t[0], block_size);
    let mut y = SparseBinMat::new(block_size, vec![y_1.non_trivial_positions().collect()]);

    for i in 1..pp.code.num_bits {
        let y_i = to_sparse_bin_vec(y_t[i], block_size);
        let y_i = SparseBinMat::new(block_size, vec![y_i.non_trivial_positions().collect()]);
        y = y.vertical_concat_with(&y_i);
    }

    let y = y.transposed();

    // Декодуємо по рядках
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
    let mut r = vec![0; pp.code.k as usize];
    for i in 0..pp.code.k as usize {
        r[i] = from_slice_to_number(decoded_mat.row(i).unwrap());
    }

    (shares.z0 - dot_product(&pp.a, &r)) as u32
}

fn encode_slice<C: AdditiveCode>(r: &SparseBinSlice, code_impl: &C) -> SparseBinMat {
    let r = SparseBinMat::new(r.len(), vec![r.non_trivial_positions().collect()]);
    code_impl.encode(&r)
}
