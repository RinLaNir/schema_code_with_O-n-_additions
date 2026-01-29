//! Integration tests for the secret sharing scheme.
//! 
//! These tests verify the complete deal → reconstruct round-trip
//! for both sequential and parallel implementations.

use ark_bls12_381::Fr;
use rand::Rng;
use rand::thread_rng;

use schema_code::types::CodeInitParams;
use schema_code::aos;
use schema_code::aos_parallel;

/// Helper function to remove random shares from a share vector.
fn remove_random_shares(shares: &mut Vec<schema_code::types::Share>, count: usize) {
    let mut rng = thread_rng();
    for _ in 0..count {
        if shares.is_empty() {
            break;
        }
        let idx = rng.gen_range(0..shares.len());
        shares.swap_remove(idx);
    }
}

/// Create default code parameters for testing.
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
        // Setup
        let params = default_test_params();
        let c_value = 10u32;
        let pp = aos::setup::<Fr>(params, c_value);
        
        // Create a secret
        let secret = Fr::from(42u64);
        
        // Deal shares
        let shares = aos::deal(&pp, secret);
        
        // Reconstruct without removing any shares
        let (reconstructed, _metrics) = aos::reconstruct(&pp, &shares);
        
        assert_eq!(secret, reconstructed, "Secret should be reconstructed correctly with no erasures");
    }

    #[test]
    fn test_deal_reconstruct_with_small_erasure() {
        // Setup
        let params = default_test_params();
        let c_value = 10u32;
        let pp = aos::setup::<Fr>(params, c_value);
        
        // Create a secret
        let secret = Fr::from(12345u64);
        
        // Deal shares
        let mut shares = aos::deal(&pp, secret);
        
        // Remove a small number of shares (within error correction capability)
        let shares_to_remove = 50;
        remove_random_shares(&mut shares.shares, shares_to_remove);
        
        // Reconstruct
        let (reconstructed, _metrics) = aos::reconstruct(&pp, &shares);
        
        assert_eq!(secret, reconstructed, 
            "Secret should be reconstructed correctly with {} erasures", shares_to_remove);
    }

    #[test]
    fn test_deal_reconstruct_different_secrets() {
        let params = default_test_params();
        let c_value = 10u32;
        let pp = aos::setup::<Fr>(params, c_value);
        
        // Test with different secret values
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
        // Setup
        let params = default_test_params();
        let c_value = 10u32;
        let pp = aos_parallel::setup::<Fr>(params, c_value);
        
        // Create a secret
        let secret = Fr::from(42u64);
        
        // Deal shares
        let shares = aos_parallel::deal(&pp, secret);
        
        // Reconstruct without removing any shares
        let (reconstructed, _metrics) = aos_parallel::reconstruct(&pp, &shares);
        
        assert_eq!(secret, reconstructed, "Secret should be reconstructed correctly with no erasures");
    }

    #[test]
    fn test_parallel_deal_reconstruct_with_small_erasure() {
        // Setup
        let params = default_test_params();
        let c_value = 10u32;
        let pp = aos_parallel::setup::<Fr>(params, c_value);
        
        // Create a secret
        let secret = Fr::from(12345u64);
        
        // Deal shares
        let mut shares = aos_parallel::deal(&pp, secret);
        
        // Remove a small number of shares
        let shares_to_remove = 50;
        remove_random_shares(&mut shares.shares, shares_to_remove);
        
        // Reconstruct
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
        let c_value = 10u32;
        
        let pp_seq = aos::setup::<Fr>(params.clone(), c_value);
        
        let params2 = default_test_params();
        let pp_par = aos_parallel::setup::<Fr>(params2, c_value);
        
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
        let c_value = 10u32;
        let pp = aos::setup::<Fr>(params, c_value);
        
        let secret = Fr::from(999u64);
        let shares = aos::deal(&pp, secret);
        
        assert_eq!(shares.shares.len(), pp.code.output_length as usize,
            "Number of shares should equal output length");
    }
}
