use sparse_bin_mat::{SparseBinSlice, SparseBinMat, SparseBinVec};
use ark_ff::{Field, PrimeField, BigInteger};
// Обчислюємо скалярний добуток у полі F
pub fn dot_product<F: Field>(a: &[F], b: &[F]) -> F {
    let mut acc = F::zero();
    for (x, y) in a.iter().zip(b) {
        acc += *x * *y;
    }
    acc
}

// Перетворення бінарного рядка (SparseBinSlice) у u32
pub fn from_slice_to_number(slice: SparseBinSlice) -> u32 {
    let mut number = 0;
    for i in 0..slice.len() {
        if slice.is_one_at(i).unwrap() {
            number += 1 << i;
        }
    }
    number
}

pub fn to_sparse_bin_vec(number: u32, len: usize) -> SparseBinVec {
    let vec = (0..len).filter(|&i| (number >> i) & 1 == 1).collect();
    SparseBinVec::new(len, vec)
}

/// Зворотна операція: з u32 в поле F
/// Знову ж таки, проста демонстрація: записуємо u32 як 64-бітне число в поле.
pub fn u32_to_field<F: PrimeField>(val: u32) -> F {
    F::from(val as u64)
}
