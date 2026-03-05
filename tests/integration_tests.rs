//! Integration tests for the secret sharing scheme.
//! 
//! These tests verify the complete deal → reconstruct round-trip
//! for both sequential and parallel implementations.

use ark_bls12_381::Fr;

use schema_code::types::CodeInitParams;
use schema_code::utils::remove_random_shares;
use schema_code::aos;
use schema_code::aos_parallel;

fn default_test_params() -> CodeInitParams {
    use ldpc_toolbox::codes::ccsds::{AR4JARate, AR4JAInfoSize};
    use ldpc_toolbox::decoder::factory::DecoderImplementation;

    CodeInitParams {
        decoder_type: Some(DecoderImplementation::Aminstarf32),
        ldpc_rate: Some(AR4JARate::R4_5),
        ldpc_info_size: Some(AR4JAInfoSize::K1024),
        max_iterations: Some(300),
        llr_value: Some(1.3863),
    }
}

mod sequential_tests {
    use super::*;

    #[test]
    fn test_deal_reconstruct_no_erasures() {
        let params = default_test_params();
        let pp = aos::setup::<Fr>(params, 10);
        let secret = Fr::from(42u64);
        let shares = aos::deal(&pp, secret);
        let (reconstructed, _metrics) = aos::reconstruct(&pp, &shares);

        assert_eq!(secret, reconstructed, "Secret should be reconstructed correctly with no erasures");
    }

    #[test]
    fn test_deal_reconstruct_with_small_erasure() {
        let params = default_test_params();
        let pp = aos::setup::<Fr>(params, 10);
        let secret = Fr::from(12345u64);
        let mut shares = aos::deal(&pp, secret);

        let shares_to_remove: isize = 50;
        remove_random_shares(&mut shares.shares, shares_to_remove);

        let (reconstructed, _metrics) = aos::reconstruct(&pp, &shares);

        assert_eq!(secret, reconstructed,
            "Secret should be reconstructed correctly with {} erasures", shares_to_remove);
    }

    #[test]
    fn test_deal_reconstruct_different_secrets() {
        let params = default_test_params();
        let pp = aos::setup::<Fr>(params, 10);

        let secrets = [
            Fr::from(0u64),
            Fr::from(1u64),
            Fr::from(u64::MAX),
            Fr::from(123456789u64),
        ];

        for secret in secrets.iter() {
            let shares = aos::deal(&pp, *secret);
            let (reconstructed, _) = aos::reconstruct(&pp, &shares);
            assert_eq!(*secret, reconstructed,
                "Failed to reconstruct secret: {:?}", secret);
        }
    }
}

mod parallel_tests {
    use super::*;

    #[test]
    fn test_parallel_deal_reconstruct_no_erasures() {
        let params = default_test_params();
        let pp = aos_parallel::setup::<Fr>(params, 10);
        let secret = Fr::from(42u64);
        let shares = aos_parallel::deal(&pp, secret);
        let (reconstructed, _metrics) = aos_parallel::reconstruct(&pp, &shares);

        assert_eq!(secret, reconstructed, "Secret should be reconstructed correctly with no erasures");
    }

    #[test]
    fn test_parallel_deal_reconstruct_with_small_erasure() {
        let params = default_test_params();
        let pp = aos_parallel::setup::<Fr>(params, 10);
        let secret = Fr::from(12345u64);
        let mut shares = aos_parallel::deal(&pp, secret);

        let shares_to_remove: isize = 50;
        remove_random_shares(&mut shares.shares, shares_to_remove);

        let (reconstructed, _metrics) = aos_parallel::reconstruct(&pp, &shares);

        assert_eq!(secret, reconstructed,
            "Secret should be reconstructed correctly with {} erasures", shares_to_remove);
    }
}

mod consistency_tests {
    use super::*;

    #[test]
    fn test_sequential_and_parallel_setup_produce_same_lengths() {
        let params = default_test_params();
        let pp_seq = aos::setup::<Fr>(params, 10);

        let params2 = default_test_params();
        let pp_par = aos_parallel::setup::<Fr>(params2, 10);

        assert_eq!(pp_seq.code.input_length, pp_par.code.input_length,
            "Input lengths should match");
        assert_eq!(pp_seq.code.output_length, pp_par.code.output_length,
            "Output lengths should match");
        assert_eq!(pp_seq.a.len(), pp_par.a.len(),
            "Coefficient vector lengths should match");
    }

    #[test]
    fn test_share_count_matches_output_length() {
        let params = default_test_params();
        let pp = aos::setup::<Fr>(params, 10);
        let secret = Fr::from(999u64);
        let shares = aos::deal(&pp, secret);

        assert_eq!(shares.shares.len(), pp.code.output_length as usize,
            "Number of shares should equal output length");
    }
}
