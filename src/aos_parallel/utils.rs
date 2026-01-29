use ark_ff::Field;
use rayon::prelude::*;

/// Calculate the dot product of two vectors in a finite field.
/// 
/// This implementation uses parallel processing for large vectors to improve
/// performance on multi-core systems. For small vectors, it falls back to
/// sequential processing to avoid parallelization overhead.
/// 
/// # Arguments
/// * `a` - First vector of field elements
/// * `b` - Second vector of field elements
/// 
/// # Returns
/// The sum of element-wise products: Σ a_i * b_i
pub fn dot_product<F: Field + Send + Sync>(a: &[F], b: &[F]) -> F {
    // Optimal chunk size for modern CPUs (balances parallelization overhead vs throughput)
    const CHUNK_SIZE: usize = 1024;
    
    if a.len() < CHUNK_SIZE {
        // For small vectors, use sequential processing (avoid parallelization overhead)
        a.iter().zip(b).fold(F::zero(), |acc, (x, y)| acc + (*x * *y))
    } else {
        // For large vectors, use parallel chunk processing
        let chunk_results: Vec<F> = a.par_chunks(CHUNK_SIZE)
            .zip(b.par_chunks(CHUNK_SIZE))
            .map(|(a_chunk, b_chunk)| {
                // Compute local sum for each chunk
                a_chunk.iter().zip(b_chunk).fold(F::zero(), |acc, (x, y)| acc + (*x * *y))
            })
            .collect();
        
        // Sum results from all chunks
        chunk_results.iter().fold(F::zero(), |acc, &x| acc + x)
    }
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
    fn test_dot_product_large_vector_parallel() {
        // Test with large vectors to trigger parallel processing
        let size = 2048;
        let a: Vec<Fr> = vec![Fr::from(2u64); size];
        let b: Vec<Fr> = vec![Fr::from(3u64); size];
        let result = dot_product(&a, &b);
        // 2 * 3 * 2048 = 12288
        assert_eq!(result, Fr::from(12288u64));
    }

    #[test]
    fn test_dot_product_sequential_vs_parallel_consistency() {
        // Ensure sequential and parallel paths give same result
        let small: Vec<Fr> = (1..100).map(|i| Fr::from(i as u64)).collect();
        let large: Vec<Fr> = (1..2000).map(|i| Fr::from(i as u64)).collect();
        
        // Small vector (sequential path)
        let small_result = dot_product(&small, &small);
        // Sum of squares 1..99 = n(n+1)(2n+1)/6 = 99*100*199/6 = 328350
        assert_eq!(small_result, Fr::from(328350u64));
        
        // Large vector (parallel path)
        let large_ones: Vec<Fr> = vec![Fr::from(1u64); large.len()];
        let large_result = dot_product(&large, &large_ones);
        // Sum 1..1999 = n(n+1)/2 = 1999*2000/2 = 1999000
        assert_eq!(large_result, Fr::from(1999000u64));
    }
}