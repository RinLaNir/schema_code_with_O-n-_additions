use sparse_bin_mat::{SparseBinSlice, SparseBinMat, SparseBinVec};

pub fn dot_product(a: &[u32], b: &[u32]) -> u64 {
    a.iter().zip(b).map(|(&x, &y)| (x as u64) * (y as u64)).sum()
}

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

pub fn get_bin_length(number: u64) -> usize {
    let mut len = 0;
    let mut n = number;
    while n > 0 {
        len += 1;
        n >>= 1;
    }
    len
}

pub fn number_vec_to_sparse_bin_vec(vec: Vec<u32>, group_max_num: u64) -> SparseBinVec {
    let mut result = SparseBinVec::new(0, Vec::new());
    let len = get_bin_length(group_max_num);
    for &num in &vec {
        let sparse_bin_vec = to_sparse_bin_vec(num, len);
        result = result.concat(&sparse_bin_vec);
    }
    result
}

pub fn sparse_bin_vec_to_number_vec(vec: SparseBinVec, group_max_num: u64) -> Vec<u32> {
    let block_size = get_bin_length(group_max_num);
    let mut result = Vec::new();
    let mut i = 0;
    while i < vec.len() {
        let mut number = 0;
        for j in 0..block_size {
            if vec.get(i + j).unwrap().is_one() {
                number += 1 << j;
            }
        }
        result.push(number);
        i += block_size;
    }
    result
}
