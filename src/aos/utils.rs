use ark_ff::Field;

/// Calculate the dot product of two vectors in a finite field.
/// 
/// # Arguments
/// * `a` - First vector of field elements
/// * `b` - Second vector of field elements
/// 
/// # Returns
/// The sum of element-wise products: Σ a_i * b_i
pub fn dot_product<F: Field>(a: &[F], b: &[F]) -> F {
    a.iter().zip(b).fold(F::zero(), |acc, (x, y)| acc + (*x * *y))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bls12_381::Fr;

    #[test]
    fn test_dot_product_zeros() {
        let a: Vec<Fr> = vec![Fr::from(0u64); 5];
        let b: Vec<Fr> = vec![Fr::from(1u64); 5];
        let result = dot_product(&a, &b);
        assert_eq!(result, Fr::from(0u64));
    }

    #[test]
    fn test_dot_product_ones() {
        let a: Vec<Fr> = vec![Fr::from(1u64); 5];
        let b: Vec<Fr> = vec![Fr::from(1u64); 5];
        let result = dot_product(&a, &b);
        assert_eq!(result, Fr::from(5u64));
    }

    #[test]
    fn test_dot_product_mixed() {
        // [1, 2, 3] · [4, 5, 6] = 1*4 + 2*5 + 3*6 = 4 + 10 + 18 = 32
        let a: Vec<Fr> = vec![Fr::from(1u64), Fr::from(2u64), Fr::from(3u64)];
        let b: Vec<Fr> = vec![Fr::from(4u64), Fr::from(5u64), Fr::from(6u64)];
        let result = dot_product(&a, &b);
        assert_eq!(result, Fr::from(32u64));
    }

    #[test]
    fn test_dot_product_empty() {
        let a: Vec<Fr> = vec![];
        let b: Vec<Fr> = vec![];
        let result = dot_product(&a, &b);
        assert_eq!(result, Fr::from(0u64));
    }

    #[test]
    fn test_dot_product_single_element() {
        let a: Vec<Fr> = vec![Fr::from(7u64)];
        let b: Vec<Fr> = vec![Fr::from(3u64)];
        let result = dot_product(&a, &b);
        assert_eq!(result, Fr::from(21u64));
    }
}