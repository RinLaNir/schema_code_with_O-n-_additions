use crate::code::AdditiveCode;
use ark_ff::Field;
use ldpc_toolbox::gf2::GF2;
use ndarray::Array1;

pub struct CodeInitParams {
    pub num_bits: usize,
    pub num_checks: usize,
    pub bit_degree: usize,
    pub check_degree: usize,
}

pub struct CodeParams<C: AdditiveCode> {
    pub output_length: u32,
    pub input_length: u32,
    pub code_impl: C
}

pub struct SecretParams<C: AdditiveCode, F: Field> {
    pub code: CodeParams<C>,
    pub a: Vec<F>,
}

pub struct Shares<F: Field> {
    pub shares: Vec<Share>,
    pub z0: F,
}

pub struct Share {
    pub y: Array1<GF2>,
    pub i: u32,
}
