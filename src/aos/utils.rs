use std::fs::File;
use ark_ff::{Field};
use ldpc_toolbox::gf2::GF2;
use ndarray::Array2;
use num_traits::{One};
use std::io::Write;

/// Calculate dot product in field F
pub fn dot_product<F: Field>(a: &[F], b: &[F]) -> F {
    let mut acc = F::zero();
    for (x, y) in a.iter().zip(b) {
        acc += *x * *y;
    }
    acc
}

fn save_matrix_to_file(matrix: &Array2<GF2>, filename: &str, nrows: usize, ncols: usize) {
    let mut file = File::create(filename).unwrap();
    for i in 0..nrows {
        for j in 0..ncols {
            let val = matrix[(i, j)];
            let val = if val.is_one() { 1 } else { 0 };
            write!(file, "{} ", val).unwrap();
        }
        write!(file, "\n").unwrap();
    }
}