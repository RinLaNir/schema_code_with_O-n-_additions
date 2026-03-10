//! Shared utility functions.

use rand::seq::SliceRandom;
use rand::SeedableRng;
use rand::rngs::StdRng;
use crate::types::Share;

/// Remove shares from a vector — supports both absolute count and percentage.
///
/// * Positive `num_to_remove` — remove that many shares
/// * Negative `num_to_remove` — treat absolute value as percentage to remove
/// * `seed` — when `Some(s)`, uses a deterministic RNG seeded with `s`
pub fn remove_random_shares(shares: &mut Vec<Share>, num_to_remove: isize, seed: Option<u64>) {
    if let Some(s) = seed {
        let mut rng = StdRng::seed_from_u64(s);
        shares.shuffle(&mut rng);
    } else {
        let mut rng = rand::rng();
        shares.shuffle(&mut rng);
    }

    let count = if num_to_remove < 0 {
        let pct = (-num_to_remove) as f64;
        (shares.len() as f64 * pct / 100.0).round() as usize
    } else {
        num_to_remove as usize
    };

    if count <= shares.len() {
        shares.truncate(shares.len() - count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::Array1;
    use ldpc_toolbox::gf2::GF2;
    use num_traits::Zero;

    fn make_shares(n: usize) -> Vec<Share> {
        (0..n).map(|i| Share { y: Array1::from_elem(1, GF2::zero()), i: i as u32 }).collect()
    }

    #[test]
    fn test_remove_absolute_count() {
        let mut shares = make_shares(10);
        remove_random_shares(&mut shares, 3, None);
        assert_eq!(shares.len(), 7);
    }

    #[test]
    fn test_remove_percentage() {
        let mut shares = make_shares(100);
        remove_random_shares(&mut shares, -25, None);
        assert_eq!(shares.len(), 75);
    }

    #[test]
    fn test_remove_zero() {
        let mut shares = make_shares(10);
        remove_random_shares(&mut shares, 0, None);
        assert_eq!(shares.len(), 10);
    }

    #[test]
    fn test_remove_all() {
        let mut shares = make_shares(5);
        remove_random_shares(&mut shares, 5, None);
        assert_eq!(shares.len(), 0);
    }

    #[test]
    fn test_remove_more_than_available_does_nothing() {
        let mut shares = make_shares(5);
        remove_random_shares(&mut shares, 10, None);
        // count (10) > len (5), so drain is not called
        assert_eq!(shares.len(), 5);
    }

    #[test]
    fn test_remove_with_seed_deterministic() {
        let mut shares1 = make_shares(20);
        remove_random_shares(&mut shares1, 5, Some(42));
        let indices1: Vec<u32> = shares1.iter().map(|s| s.i).collect();

        let mut shares2 = make_shares(20);
        remove_random_shares(&mut shares2, 5, Some(42));
        let indices2: Vec<u32> = shares2.iter().map(|s| s.i).collect();

        assert_eq!(indices1, indices2, "Same seed must produce identical removal patterns");
    }

    #[test]
    fn test_remove_different_seeds_differ() {
        let mut shares1 = make_shares(20);
        remove_random_shares(&mut shares1, 5, Some(1));
        let indices1: Vec<u32> = shares1.iter().map(|s| s.i).collect();

        let mut shares2 = make_shares(20);
        remove_random_shares(&mut shares2, 5, Some(2));
        let indices2: Vec<u32> = shares2.iter().map(|s| s.i).collect();

        assert_ne!(indices1, indices2, "Different seeds should produce different removal patterns");
    }
}
