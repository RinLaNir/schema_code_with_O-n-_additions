use ldpc::codes::LinearCode;
use ldpc::decoders::{BpDecoder, LinearDecoder};
use ldpc::noise::Probability;
use rand::{random, Rng, SeedableRng};
use rand::rngs::StdRng;
use sparse_bin_mat::{SparseBinMat, SparseBinSlice, SparseBinVec};

/// `CodeParams` is a structure that holds parameters related to a linear code.
///
/// # Fields
///
/// * `k` - A `u32` that represents the dimension of the code, i.e., the number of information bits.
/// * `num_bits` - A `usize` that represents the total number of bits in the code, including both information and parity bits.
/// * `code` - An instance of `LinearCode` that represents the linear code used for encoding and decoding.
/// * `decoder` - An instance of `BpDecoder` that represents the Belief Propagation decoder used for decoding the code.
/// * `enc` - A function pointer to an encoding function. This function takes a `SparseBinSlice` and a `LinearCode` as input and returns a `SparseBinMat` as output.
/// * `dec` - A function pointer to a decoding function. This function takes a `BpDecoder`, a `SparseBinSlice`, and a `u32` as input and returns a `SparseBinVec` as output.
struct CodeParams {
    k: u32,
    num_bits: usize,
    code: LinearCode,
    decoder: BpDecoder,
}

/// `SecretParams` is a structure that holds the public parameters for the secret sharing scheme.
///
/// # Fields
///
/// * `code` - An instance of `CodeParams` that represents the parameters of the linear code used for encoding and decoding.
/// * `a` - A `Vec<u32>` that represents the coefficients of the polynomial used in the secret sharing scheme.
struct SecretParams {
    code: CodeParams,
    a: Vec<u32>,
}

/// `Shares` is a structure that holds the shares for the secret sharing scheme.
///
/// # Fields
///
/// * `shares` - A `Vec<Share>` that represents the shares of the secret. Each `Share` contains a part of the secret and an index.
/// * `z0` - A `u64` that represents the sum of the secret and the dot product of the polynomial coefficients and the random numbers.
struct Shares {
    shares: Vec<Share>,
    z0: u64,
}

/// `Share` is a structure that represents a share in the secret sharing scheme.
///
/// # Fields
///
/// * `y` - A `u32` that represents a part of the secret.
/// * `i` - A `u32` that represents the index of the share.
struct Share {
    y: u32,
    i: u32,
}

/// `CodeInitParams` is a structure that holds the initialization parameters for a linear code.
///
/// # Fields
///
/// * `num_bits` - A `usize` that represents the total number of bits in the code, including both information and parity bits.
/// * `num_checks` - A `usize` that represents the number of checks to be performed in the code.
/// * `bit_degree` - A `usize` that represents the degree of each bit in the code.
/// * `check_degree` - A `usize` that represents the degree of each check in the code.
struct CodeInitParams {
    num_bits: usize,
    num_checks: usize,
    bit_degree: usize,
    check_degree: usize,
}

/// Sets up the `SecretParams` for the secret sharing scheme.
///
/// # Arguments
///
/// * `cp` - An instance of `CodeInitParams` that holds the initialization parameters for a linear code.
/// * `c` - A `u32` that represents the upper limit for the random generation of the coefficients of the polynomial used in the secret sharing scheme.
///
/// # Returns
///
/// * An instance of `SecretParams` that holds the public parameters for the secret sharing scheme.
///
/// # Process
///
/// * A random regular code is generated using the parameters from `cp` and a fixed seed for reproducibility.
/// * The dimension `k` of the code is determined from the number of rows in the generator matrix of the code.
/// * A vector `a` of `k` random coefficients is generated, each in the range from 0 to `c`.
/// * A `BpDecoder` is created using the parity check matrix of the code and a fixed probability and maximum number of iterations.
/// * The `SecretParams` are returned, containing the `CodeParams` and the vector `a`.
fn setup(cp: CodeInitParams, c: u32) -> SecretParams {
    let code = LinearCode::random_regular_code()
        .num_bits(cp.num_bits)
        .num_checks(cp.num_checks)
        .bit_degree(cp.bit_degree)
        .check_degree(cp.check_degree)
        .sample_with(&mut StdRng::seed_from_u64(123))
        .unwrap();

    let k = code.generator_matrix().number_of_rows() as u32;
    let a: Vec<u32> = (0..k).map(|_| rand::thread_rng().gen_range(0..c)).collect();
    let decoder = BpDecoder::new(code.parity_check_matrix(), Probability::new(0.1), 10);

    SecretParams {
        code: CodeParams {
            k,
            num_bits: cp.num_bits,
            code,
            decoder,
        },
        a,
    }
}

/// Generates the `Shares` for the secret sharing scheme.
///
/// # Arguments
///
/// * `pp` - An instance of `SecretParams` that holds the public parameters for the secret sharing scheme.
/// * `s` - A `u32` that represents the secret to be shared.
///
/// # Returns
///
/// * An instance of `Shares` that holds the shares of the secret.
///
/// # Process
///
/// * A vector `r` of `k` random numbers is generated, where `k` is the dimension of the code.
/// * The sum `z0` of the secret `s` and the dot product of the polynomial coefficients and the random numbers is calculated.
/// * A `SparseBinMat` `r` is created, where each row represents a random number in binary form.
/// * The rows of `r` are encoded using the linear code to create a `SparseBinMat` `y`.
/// * The rows of `y` are converted to numbers and paired with their indices to create a vector of tuples.
/// * The tuples are converted to `Share`s and returned as part of the `Shares`.
fn deal(pp: &SecretParams, s: u32) -> Shares {
    let mut r = vec![0; pp.code.k as usize];
    for i in 0..pp.code.k as usize {
        r[i] = random();
    }

    let z0 = s as u64 + dot_product(&pp.a, &r);
    let block_size = 32;

    let rows: Vec<Vec<usize>> = (0..pp.code.k).map(|i| {
        let mut row = Vec::new();
        for j in 0..block_size {
            if (r[i as usize] >> j) & 1 == 1 {
                row.push(j);
            }
        }
        row
    }).collect();

    let r = SparseBinMat::new(block_size, rows).transposed();

    let mut y = encode_slice(&r.row(0).unwrap(), &pp.code.code);

    for i in 1..block_size {
        let row = r.row(i).unwrap();
        let y_i = encode_slice(&row, &pp.code.code);
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

/// Reconstructs the secret from the shares.
///
/// # Arguments
///
/// * `pp` - An instance of `SecretParams` that holds the public parameters for the secret sharing scheme.
/// * `shares` - An instance of `Shares` that holds the shares of the secret.
///
/// # Returns
///
/// * A `u32` that represents the reconstructed secret.
///
/// # Process
///
/// * A vector `y_t` is created, where each element is the `y` value of a `Share` in `shares`.
/// * A `SparseBinMat` `y` is created, where each row represents a `y_t` value in binary form.
/// * The rows of `y` are decoded using the linear code to create a `SparseBinMat` `decoded_mat`.
/// * The rows of `decoded_mat` are converted to numbers to create a vector `r`.
/// * The secret is reconstructed as the difference between `z0` in `shares` and the dot product of the polynomial coefficients and `r`.
fn reconstruct(pp: &SecretParams, shares: &Shares) -> u32 {
    let mut y_t = vec![0; pp.code.num_bits];
    for share in &shares.shares {
        y_t[share.i as usize] = share.y;
    }

    let block_size = 32;
    let y_1 = to_sparse_bin_vec(y_t[0], block_size);
    let mut y = SparseBinMat::new(block_size, vec![y_1.non_trivial_positions().collect()]);

    for i in 1..pp.code.num_bits {
        let y_i = to_sparse_bin_vec(y_t[i as usize], block_size);
        let y_i = SparseBinMat::new(block_size, vec![y_i.non_trivial_positions().collect()]);
        y = y.vertical_concat_with(&y_i);
    }

    let y = y.transposed();
    let y_1 = y.row(0).unwrap();
    let message = decode(&pp.code.decoder, y_1, &pp.code.k);

    let mut decoded_mat = SparseBinMat::new(pp.code.num_bits as usize, vec![message.non_trivial_positions().collect()]);

    for i in 1..block_size {
        let y_i = y.row(i).unwrap();
        let y_i = decode(&pp.code.decoder, y_i, &pp.code.k);
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

/// Calculates the dot product of two vectors.
///
/// # Arguments
///
/// * `a` - A slice of `u32` that represents the first vector.
/// * `b` - A slice of `u32` that represents the second vector.
///
/// # Returns
///
/// * A `u64` that represents the dot product of `a` and `b`.
///
/// # Process
///
/// * The elements of `a` and `b` are paired together.
/// * Each pair of elements is multiplied together.
/// * The products are summed to calculate the dot product.
fn dot_product(a: &[u32], b: &[u32]) -> u64 {
    a.iter().zip(b).map(|(&x, &y)| (x as u64) * (y as u64)).sum()
}

/// Encodes a `SparseBinSlice` using a `LinearCode`.
///
/// # Arguments
///
/// * `r` - A reference to a `SparseBinSlice` that represents the slice to be encoded.
/// * `code` - A reference to a `LinearCode` that represents the linear code used for encoding.
///
/// # Returns
///
/// * A `SparseBinMat` that represents the encoded slice.
///
/// # Process
///
/// * A `SparseBinMat` `r` is created from the input slice, where each row represents a non-trivial position in the slice.
/// * The `SparseBinMat` `r` is multiplied with the generator matrix of the `LinearCode` to encode the slice.
fn encode_slice(r: &SparseBinSlice, code: &LinearCode) -> SparseBinMat {
    let r = SparseBinMat::new(r.len(), vec![r.non_trivial_positions().collect()]);
    &r * code.generator_matrix()
}

/// Decodes a `SparseBinSlice` using a `BpDecoder`.
///
/// # Arguments
///
/// * `dec` - A reference to a `BpDecoder` that represents the Belief Propagation decoder used for decoding.
/// * `erasure_part` - A `SparseBinSlice` that represents the part of the code to be decoded.
/// * `k` - A reference to a `u32` that represents the dimension of the code, i.e., the number of information bits.
///
/// # Returns
///
/// * A `SparseBinVec` that represents the decoded slice.
///
/// # Process
///
/// * The rightmost position `right_pos` in the slice is determined from the length of the slice.
/// * The leftmost position `left_pos` in the slice is determined as the difference between `right_pos` and `k`.
/// * A vector `to_keep` of positions to be kept in the decoded slice is created, ranging from `left_pos` to `right_pos`.
/// * The `SparseBinSlice` `erasure_part` is decoded using the `BpDecoder` `dec` to create a `SparseBinVec` `decoded`.
/// * The positions in `decoded` that are not in `to_keep` are discarded.
/// * The `SparseBinVec` `decoded` is returned.
fn decode(dec: &BpDecoder, erasure_part: SparseBinSlice, k: &u32) -> SparseBinVec {
    let right_pos = erasure_part.len();
    let left_pos = right_pos - *k as usize;
    let to_keep: Vec<usize> = (left_pos..right_pos).collect();
    let decoded = dec.decode(erasure_part);
    decoded.keep_only_positions(&to_keep).unwrap()
}

/// Converts a number into a `SparseBinVec`.
///
/// # Arguments
///
/// * `number` - A `u32` that represents the number to be converted.
/// * `len` - A `usize` that represents the length of the binary representation of the number.
///
/// # Returns
///
/// * A `SparseBinVec` that represents the binary form of the number.
///
/// # Process
///
/// * A vector `vec` is created, where each element is a position `i` in the binary representation of `number` where the bit is 1.
/// * A `SparseBinVec` is created from `vec`, with length `len`.
fn to_sparse_bin_vec(number: u32, len: usize) -> SparseBinVec {
    let vec = (0..len).filter(|&i| (number >> i) & 1 == 1).collect();
    SparseBinVec::new(len, vec)
}

/// Calculates the binary length of a number.
///
/// # Arguments
///
/// * `number` - A `u64` that represents the number for which the binary length is to be calculated.
///
/// # Returns
///
/// * A `usize` that represents the binary length of `number`.
///
/// # Process
///
/// * A variable `len` is initialized to 0 to keep track of the binary length.
/// * A copy `n` of `number` is created.
/// * While `n` is greater than 0, `len` is incremented and `n` is right-shifted by 1 bit.
/// * The binary length `len` is returned.
fn get_bin_length(number: u64) -> usize {
    let mut len = 0;
    let mut n = number;
    while n > 0 {
        len += 1;
        n >>= 1;
    }
    len
}

/// Converts a `SparseBinSlice` into a number.
///
/// # Arguments
///
/// * `slice` - A `SparseBinSlice` that represents the binary form of the number.
///
/// # Returns
///
/// * A `u32` that represents the number converted from the binary form.
///
/// # Process
///
/// * A variable `number` is initialized to 0 to keep track of the number.
/// * For each position `i` in the slice, if the bit at `i` is 1, `number` is incremented by 2 raised to the power of `i`.
/// * The number `number` is returned.
fn from_slice_to_number(slice: SparseBinSlice) -> u32 {
    let mut number = 0;
    for i in 0..slice.len() {
        if slice.is_one_at(i).unwrap() {
            number += 1 << i;
        }
    }
    number
}

/// Converts a vector of numbers into a `SparseBinVec`.
///
/// # Arguments
///
/// * `vec` - A `Vec<u32>` that represents the vector of numbers to be converted.
/// * `group_max_num` - A `u64` that represents the maximum number in the group. This is used to determine the binary length of the numbers.
///
/// # Returns
///
/// * A `SparseBinVec` that represents the binary form of the numbers.
///
/// # Process
///
/// * A `SparseBinVec` `result` is initialized with length 0 and no non-trivial positions.
/// * The binary length `len` of the numbers is determined from `group_max_num`.
/// * For each number `num` in `vec`, a `SparseBinVec` `sparse_bin_vec` is created from `num` with length `len`.
/// * The `SparseBinVec` `sparse_bin_vec` is concatenated to `result`.
/// * The `SparseBinVec` `result` is returned.
fn number_vec_to_sparse_bin_vec(vec: Vec<u32>, group_max_num: u64) -> SparseBinVec {
    let mut result = SparseBinVec::new(0, Vec::new());
    let len = get_bin_length(group_max_num);
    for &num in &vec {
        let sparse_bin_vec = to_sparse_bin_vec(num, len);
        result = result.concat(&sparse_bin_vec);
    }
    result
}

/// Converts a `SparseBinVec` into a vector of numbers.
///
/// # Arguments
///
/// * `vec` - A `SparseBinVec` that represents the binary form of the numbers.
/// * `group_max_num` - A `u64` that represents the maximum number in the group. This is used to determine the binary length of the numbers.
///
/// # Returns
///
/// * A `Vec<u32>` that represents the numbers converted from the binary form.
///
/// # Process
///
/// * The binary length `block_size` of the numbers is determined from `group_max_num`.
/// * A vector `result` is initialized to store the numbers.
/// * A variable `i` is initialized to 0 to keep track of the current position in `vec`.
/// * While `i` is less than the length of `vec`, a number is calculated from the bits in `vec` from position `i` to `i + block_size`.
/// * The number is added to `result` and `i` is incremented by `block_size`.
/// * The vector `result` is returned.
fn sparse_bin_vec_to_number_vec(vec: SparseBinVec, group_max_num: u64) -> Vec<u32> {
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

fn main() {
    let secret = 42;
    let c = 10;
    let code_params = CodeInitParams {
        num_bits: 16,
        num_checks: 12,
        bit_degree: 3,
        check_degree: 4,
    };
    let pp = setup(code_params, c);
    let mut shares = deal(&pp, secret);
    
    shares.shares.pop();
    shares.shares.pop();
    
    let reconstructed_secret = reconstruct(&pp, &shares);

    println!("Original Secret: {}", secret);
    println!("Reconstructed Secret: {}", reconstructed_secret);
}