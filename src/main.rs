use ldpc::codes::LinearCode;
use ldpc::decoders::{BpDecoder, LinearDecoder};
use ldpc::noise::{Probability};
use rand::{random, Rng, SeedableRng};
use rand::rngs::StdRng;
use sparse_bin_mat::{SparseBinMat, SparseBinVec};

struct CodeParams {
    n: usize,
    k: usize,
    enc: fn(&SparseBinMat, &LinearCode) -> SparseBinMat,
    dec: fn(&[Option<u32>], &mut [u32]),
}

struct PublicParams {
    code: CodeParams,
    a: Vec<u32>,
}

struct Shares {
    y: Vec<u32>,
    z0: u64,
}

fn setup(n: usize, rate: f32, c: u32) -> PublicParams {
    let k = (rate * n as f32).ceil() as usize;
    let a: Vec<u32> = (0..k).map(|_| rand::thread_rng().gen_range(0..c)).collect();

    PublicParams {
        code: CodeParams {
            n,
            k,
            enc: encode,
            dec: decode,
        },
        a
    }
}

fn deal(pp: &PublicParams, s: u32) -> Shares {
    let mut r = vec![0; pp.code.k];
    for i in 0..pp.code.k {
        r[i] = random();
    }

    let mut y = vec![0; pp.code.n];
    //(pp.code.enc)(&r, &mut y);

    let z0 = s as u64 + dot_product(&pp.a, &r);

    Shares { y, z0 }
}

fn reconstruct(pp: &PublicParams, shares: &Shares, t: &[usize]) -> u32 {
    let mut y_t = vec![None; pp.code.n];
    for &i in t {
        y_t[i] = Some(shares.y[i]);
    }

    let mut r = vec![0; y_t.len()];
    (pp.code.dec)(&y_t, &mut r);

    (shares.z0 - dot_product(&pp.a, &r)) as u32
}

fn dot_product(a: &[u32], b: &[u32]) -> u64 {
    a.iter().zip(b).map(|(&x, &y)| (x as u64) * (y as u64)).sum()
}

// TODO: Placeholder linear erasure code encoder
fn encode(r: &SparseBinMat, code: &LinearCode) -> SparseBinMat {
    println!("R: {}", r);
    let erasure_part = r * code.generator_matrix();
    erasure_part
}

// TODO: Placeholder linear erasure code decoder
fn decode(y_t: &[Option<u32>], r: &mut [u32]) {
    for (i, y) in y_t.iter().enumerate().take(r.len()) {
        if let Some(y_i) = y {
            r[i] = *y_i;
        }
    }
}

// fn to_llrs(bits: &Array1<GF2>) -> Vec<f64> {
//     bits.iter()
//         .map(|&b| if b == GF2::zero() { 1.3863 } else { -1.3863 })
//         .collect()
// }
//
// fn to_u8(bits: &Array1<GF2>) -> Vec<u8> {
//     bits.iter().map(|&b| if b == GF2::zero() { 0 } else { 1 }).collect()
// }

// number to SparseBinVec
fn to_sparse_bin_vec(number: u32, len: usize) -> SparseBinVec {
    let vec = (0..len).filter(|&i| (number >> i) & 1 == 1).collect();
    SparseBinVec::new(len, vec)
}

fn get_bin_length(number: u32) -> usize {
    let mut len = 0;
    let mut n = number;
    while n > 0 {
        len += 1;
        n >>= 1;
    }
    len
}

// SparseBinVec to number
fn from_sparse_bin_vec(vec: SparseBinVec) -> u32 {
    let mut number = 0;
    let positions = vec.to_positions_vec();
    for i in 0..positions.len() {
        number += 1 << positions[i];
    }
    number
}

fn number_vec_to_sparse_bin_vec(vec: Vec<u32>, group_max_num: u32) -> SparseBinVec {
    let mut result = SparseBinVec::new(0, Vec::new());
    let len = get_bin_length(group_max_num);
    for i in 0..vec.len() {
        let sparse_bin_vec = to_sparse_bin_vec(vec[i], len);
        result = result.concat(&sparse_bin_vec);
    }
    result
}

fn sparse_bin_vec_to_number_vec(vec: SparseBinVec, group_max_num: u32) -> Vec<u32> {
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

// make config struct for threshold cryptography scheme
struct ThresholdConfig {
    n: usize,
    rate: f32,
    c: u32,
    t: usize,
}


fn main() {
    let n = 6;
    let secret = 42;
    let rate = 0.5;
    let c = 10;

    let pp = setup(n, rate, c);
    let shares = deal(&pp, secret);

    let t = (0..n).collect::<Vec<usize>>();
    let reconstructed_secret = reconstruct(&pp, &shares, &t);

    println!("Original Secret: {}", secret);
    // >> Original Secret: 42
    println!("Reconstructed Secret: {}", reconstructed_secret);
    // >> Reconstructed Secret: 42

    //let mut rng = rand::thread_rng();
    // let code = LinearCode::random_regular_code()
    //     .num_bits(4)
    //     .num_checks(3)
    //     .bit_degree(3)
    //     .check_degree(4)
    //     .sample_with(&mut rng)
    //     .unwrap();

    let code = LinearCode::random_regular_code()
        .num_bits(16)
        .num_checks(12)
        .bit_degree(3)
        .check_degree(4)
        .sample_with(&mut StdRng::seed_from_u64(123))
        .unwrap();

    println!("Generator matrix:");
    println!("{}", code.generator_matrix());
    println!("Parity check matrix:");
    println!("{}", code.parity_check_matrix());
    println!("Length: {}", code.len());

    //let noise = BinarySymmetricChannel::with_probability(Probability::new(0.2));
    //let error = code.random_error(&noise, &mut thread_rng());
    //println!("{}", error);

    // let parity_check_matrix = SparseBinMat::new(
    //     7,
    //     vec![vec![0, 1, 2, 4], vec![0, 1, 3, 5], vec![0, 2, 3, 6]]
    // );
    // let code = LinearCode::from_parity_check_matrix(parity_check_matrix);
    // 
    // let noise = BinarySymmetricChannel::with_probability(Probability::new(0.25));
    // let error = code.random_error(&noise, &mut thread_rng());
    // 
    // assert_eq!(error.len(), 7);
    // //println!("{}", error);

    let decoder = BpDecoder::new(code.parity_check_matrix(), Probability::new(0.1), 10);
    let message = SparseBinMat::new(4, vec![vec![0, 1, 2]]);
    let codeword = encode(&message, &code);
    println!("Code word: {}", codeword);
    let error = SparseBinMat::new(code.len(), vec![vec![0]]);
    println!("{}", error);
    let corrupted = &codeword + &error;
    println!("{}", corrupted);
    let decoded = decoder.decode(corrupted.row(0).unwrap());
    println!("{}", decoded.as_view());
    assert_eq!(decoded.as_view(), codeword.row(0).unwrap().as_view());

    // let h = ldpc_toolbox::codes::dvbs2::Code::R3_4.h();
    // let conf = ldpc_toolbox::mackay_neal::Config {
    //     nrows: 4,
    //     ncols: 8,
    //     wr: 6,
    //     wc: 3,
    //     backtrack_cols: 0,
    //     backtrack_trials: 0,
    //     min_girth: None,
    //     girth_trials: 0,
    //     fill_policy: ldpc_toolbox::mackay_neal::FillPolicy::Random,
    // };
    // let h = conf.run(9).unwrap();
    // print!("{}", h.alist());
    // //println!("{}", seed);
    // println!("{}", h.num_rows());
    // println!("{}", h.num_cols());
    // // save alist to txt
    // // let alist = h.alist();
    // // write("alist.txt", alist).unwrap();
    //
    // let encoder = ldpc_toolbox::encoder::Encoder::from_h(&h).unwrap();
    //
    // let i = ldpc_toolbox::gf2::GF2::one();
    // let o = ldpc_toolbox::gf2::GF2::zero();
    //
    // let message = (0..4).map(|x| if x % 2 == 0 { i } else { o }).collect::<Vec<_>>();
    // // print len
    // println!("{}", message.len());
    // println!("{:?}", message);
    // let codeword = encoder.encode(&ndarray::arr1(&message));
    // println!("{:?}", codeword.len());
    // // print codeword
    // //println!("{:?}", codeword);
    // // print message
    // //println!("{:?}", message);
    //
    // let mut decoder = ldpc_toolbox::decoder::flooding::Decoder::new(h, ldpc_toolbox::decoder::arithmetic::Aminstari8::new());
    // let DecoderOutput {
    //     codeword: decoded,
    //     iterations,
    // } = decoder.decode(&to_llrs(&codeword), 100).unwrap();
    //
    // // print decoded
    // //println!("{:?}", decoded);
    // assert_eq!(decoded.len(), codeword.len());
    // println!("Success");
    // // print iterations
    // println!("{:?}", iterations);
    //
    // for j in 0..codeword.len() {
    //     let mut codeword_bad = codeword.clone();
    //     codeword_bad[j] += GF2::one();
    //     println!("{:?}", codeword_bad);
    //     let max_iter = 100;
    //     let output = decoder.decode(&to_llrs(&codeword_bad), max_iter);
    //     match output {
    //         Ok(DecoderOutput { codeword: decoded, iterations }) => {
    //             assert_eq!(&decoded, &to_u8(&codeword));
    //             assert_eq!(iterations, 1);
    //             println!("Success {}", j);
    //         }
    //         Err(_e) => {
    //             eprintln!("Decoding failed.");
    //         }
    //     }
    // }
}