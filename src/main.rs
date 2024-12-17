mod types;
mod code;
mod aos;

use aos::{setup, deal, reconstruct};

fn main() {
    let secret = 42;
    let c = 10;

    let code_params = types::CodeInitParams {
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
