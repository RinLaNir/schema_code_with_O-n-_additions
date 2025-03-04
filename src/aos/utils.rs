use sparse_bin_mat::{SparseBinSlice, SparseBinMat, SparseBinVec};
use ark_ff::{Field, PrimeField, BigInteger};
use ldpc_toolbox::gf2::GF2;
use ndarray::{Array1, ArrayView1};
use num_traits::{One, Zero};

// Обчислюємо скалярний добуток у полі F
pub fn dot_product<F: Field>(a: &[F], b: &[F]) -> F {
    let mut acc = F::zero();
    for (x, y) in a.iter().zip(b) {
        acc += *x * *y;
    }
    acc
}

pub fn from_slice_to_number(slice: ArrayView1<GF2>) -> u32 {
    let mut number = 0;
    for i in 0..slice.len() {
        if slice[i].is_one() {
            number += 1 << i;
        }
    }
    number
}

pub fn from_number_to_slice(number: u32, len: usize) -> Array1<GF2> {
    let vec: Vec<GF2> = (0..len)
        .map(|i| if (number >> i) & 1 == 1 { GF2::one() } else { GF2::zero() })
        .collect();
    Array1::from(vec)
}

/// Зворотна операція: з u32 в поле F
/// Знову ж таки, проста демонстрація: записуємо u32 як 64-бітне число в поле.
pub fn u32_to_field<F: PrimeField>(val: u32) -> F {
    F::from(val as u64)
}
