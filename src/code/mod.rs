pub mod ldpc_impl;

use sparse_bin_mat::{SparseBinMat, SparseBinSlice, SparseBinVec};

pub trait AdditiveCode {
    fn setup(params: crate::types::CodeInitParams) -> Self;
    fn encode(&self, input: &SparseBinMat) -> SparseBinMat;
    fn decode(&self, input: SparseBinSlice) -> SparseBinVec;
    fn generator_matrix(&self) -> &SparseBinMat;
    fn k(&self) -> u32;
}