use rand::{random, Rng};

struct CodeParams {
    n: usize,
    k: usize,
    enc: fn(&[u32], &mut [u32]),
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
    (pp.code.enc)(&r, &mut y);

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
fn encode(r: &[u32], y: &mut [u32]) {
    y[..r.len()].copy_from_slice(r);
}

// TODO: Placeholder linear erasure code decoder
fn decode(y_t: &[Option<u32>], r: &mut [u32]) {
    for (i, y) in y_t.iter().enumerate().take(r.len()) {
        if let Some(y_i) = y {
            r[i] = *y_i;
        }
    }
}

fn main() {
    let n = 6;
    let secret = 42;
    let rate = 0.5;
    let c = 10;

    let pp = setup(n, rate, c);

    let shares = deal(&pp, secret);

    // let t = vec![0, 2, 4, 5]; // Indices of shares used for reconstruction
    let t = (0..n).collect::<Vec<usize>>();
    let reconstructed_secret = reconstruct(&pp, &shares, &t);

    println!("Original Secret: {}", secret);
    println!("Reconstructed Secret: {}", reconstructed_secret);
}