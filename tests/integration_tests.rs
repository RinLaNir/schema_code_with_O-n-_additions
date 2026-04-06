//! Integration tests for the secret sharing scheme.

use schema_code::aos;
use schema_code::aos_parallel;
use schema_code::types::{CodeInitParams, F2PowElement};
use schema_code::utils::remove_random_shares;

fn default_test_params() -> CodeInitParams {
    use ldpc_toolbox::codes::ccsds::{AR4JAInfoSize, AR4JARate};
    use ldpc_toolbox::decoder::factory::DecoderImplementation;

    CodeInitParams {
        decoder_type: Some(DecoderImplementation::Aminstarf32),
        ldpc_rate: Some(AR4JARate::R4_5),
        ldpc_info_size: Some(AR4JAInfoSize::K1024),
        max_iterations: Some(300),
        llr_value: Some(1.3863),
        secret_bits: Some(128),
    }
}

fn secret(hex: &str) -> F2PowElement {
    F2PowElement::from_hex(hex, 128).unwrap()
}

mod sequential_tests {
    use super::*;

    #[test]
    fn test_deal_reconstruct_no_erasures() {
        let params = default_test_params();
        let pp = aos::setup(params);
        let secret = super::secret("2a");
        let shares = aos::deal(&pp, &secret);
        let (reconstructed, _metrics) = aos::reconstruct(&pp, &shares);

        assert_eq!(Some(secret), reconstructed);
    }

    #[test]
    fn test_deal_reconstruct_with_small_erasure() {
        let params = default_test_params();
        let pp = aos::setup(params);
        let secret = super::secret("3039");
        let mut shares = aos::deal(&pp, &secret);

        remove_random_shares(&mut shares.shares, 50, None);
        let (reconstructed, _metrics) = aos::reconstruct(&pp, &shares);

        assert_eq!(Some(secret), reconstructed);
    }

    #[test]
    fn test_deal_reconstruct_different_secrets() {
        let params = default_test_params();
        let pp = aos::setup(params);

        let secrets = [
            super::secret("00"),
            super::secret("01"),
            super::secret("ffffffffffffffff"),
            super::secret("1234567890abcdef1234"),
        ];

        for secret in secrets {
            let shares = aos::deal(&pp, &secret);
            let (reconstructed, _) = aos::reconstruct(&pp, &shares);
            assert_eq!(Some(secret), reconstructed);
        }
    }
}

mod parallel_tests {
    use super::*;

    #[test]
    fn test_parallel_deal_reconstruct_no_erasures() {
        let params = default_test_params();
        let pp = aos_parallel::setup(params);
        let secret = super::secret("2a");
        let shares = aos_parallel::deal(&pp, &secret);
        let (reconstructed, _metrics) = aos_parallel::reconstruct(&pp, &shares);

        assert_eq!(Some(secret), reconstructed);
    }

    #[test]
    fn test_parallel_deal_reconstruct_with_small_erasure() {
        let params = default_test_params();
        let pp = aos_parallel::setup(params);
        let secret = super::secret("3039");
        let mut shares = aos_parallel::deal(&pp, &secret);

        remove_random_shares(&mut shares.shares, 50, None);
        let (reconstructed, _metrics) = aos_parallel::reconstruct(&pp, &shares);

        assert_eq!(Some(secret), reconstructed);
    }
}

mod consistency_tests {
    use super::*;

    #[test]
    fn test_sequential_and_parallel_setup_produce_same_lengths() {
        let params = default_test_params();
        let pp_seq = aos::setup(params);

        let params2 = default_test_params();
        let pp_par = aos_parallel::setup(params2);

        assert_eq!(pp_seq.code.input_length, pp_par.code.input_length);
        assert_eq!(pp_seq.code.output_length, pp_par.code.output_length);
        assert_eq!(pp_seq.a_bits.len(), pp_par.a_bits.len());
        assert_eq!(pp_seq.ell, 128);
    }

    #[test]
    fn test_share_count_matches_output_length() {
        let params = default_test_params();
        let pp = aos::setup(params);
        let secret = super::secret("03e7");
        let shares = aos::deal(&pp, &secret);

        assert_eq!(shares.shares.len(), pp.code.output_length as usize);
    }

    #[test]
    fn test_setup_generates_binary_mask_of_length_k() {
        let params = default_test_params();
        let pp = aos::setup(params);

        assert_eq!(pp.a_bits.len(), pp.code.input_length as usize);
        assert!(pp.code.input_length as usize >= pp.ell);
    }

    #[test]
    fn test_reconstruct_returns_none_when_decoding_fails() {
        let params = default_test_params();
        let pp = aos::setup(params);
        let secret = super::secret("deadbeef");
        let mut shares = aos::deal(&pp, &secret);
        shares.shares.clear();

        let (reconstructed, _metrics) = aos::reconstruct(&pp, &shares);
        assert!(reconstructed.is_none());
    }
}
