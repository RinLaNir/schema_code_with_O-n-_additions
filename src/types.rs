use crate::code::AdditiveCode;

pub struct CodeInitParams {
    pub num_bits: usize,
    pub num_checks: usize,
    pub bit_degree: usize,
    pub check_degree: usize,
}

pub struct CodeParams<C: AdditiveCode> {
    pub k: u32,
    pub num_bits: usize,
    pub code_impl: C,
}

pub struct SecretParams<C: AdditiveCode> {
    pub code: CodeParams<C>,
    pub a: Vec<u32>,
}

pub struct Shares {
    pub shares: Vec<Share>,
    pub z0: u64,
}

pub struct Share {
    pub y: u32,
    pub i: u32,
}
