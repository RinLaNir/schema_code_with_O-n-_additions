//! Shared utility functions.

use rand::seq::SliceRandom;
use crate::types::Share;

/// Remove shares from a vector — supports both absolute count and percentage.
///
/// * Positive `num_to_remove` — remove that many shares
/// * Negative `num_to_remove` — treat absolute value as percentage to remove
pub fn remove_random_shares(shares: &mut Vec<Share>, num_to_remove: isize) {
    let mut rng = rand::rng();
    shares.shuffle(&mut rng);

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
        remove_random_shares(&mut shares, 3);
        assert_eq!(shares.len(), 7);
    }

    #[test]
    fn test_remove_percentage() {
        let mut shares = make_shares(100);
        remove_random_shares(&mut shares, -25);
        assert_eq!(shares.len(), 75);
    }

    #[test]
    fn test_remove_zero() {
        let mut shares = make_shares(10);
        remove_random_shares(&mut shares, 0);
        assert_eq!(shares.len(), 10);
    }

    #[test]
    fn test_remove_all() {
        let mut shares = make_shares(5);
        remove_random_shares(&mut shares, 5);
        assert_eq!(shares.len(), 0);
    }

    #[test]
    fn test_remove_more_than_available_does_nothing() {
        let mut shares = make_shares(5);
        remove_random_shares(&mut shares, 10);
        // count (10) > len (5), so drain is not called
        assert_eq!(shares.len(), 5);
    }
}
