/// Computes the arithmetic mean of a slice of values. Returns 0.0 for empty input.
pub fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

/// Computes the population standard deviation given a pre-computed mean.
/// Returns 0.0 for empty input.
pub fn stddev(values: &[f64], mean: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;

    variance.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mean_empty() {
        assert_eq!(mean(&[]), 0.0);
    }

    #[test]
    fn test_mean_single() {
        assert_eq!(mean(&[7.0]), 7.0);
    }

    #[test]
    fn test_mean_values() {
        assert_eq!(mean(&[1.0, 2.0, 3.0]), 2.0);
    }

    #[test]
    fn test_stddev_empty() {
        assert_eq!(stddev(&[], 0.0), 0.0);
    }

    #[test]
    fn test_stddev_uniform() {
        assert_eq!(stddev(&[5.0, 5.0, 5.0], 5.0), 0.0);
    }

    #[test]
    fn test_stddev_known() {
        // values [2,4,4,4,5,5,7,9], mean=5.0, population stddev=2.0
        let vals = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        assert!((stddev(&vals, 5.0) - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_stddev_single_value() {
        // A single element has no spread — stddev is 0.0
        assert_eq!(stddev(&[42.0], 42.0), 0.0);
    }

    #[test]
    fn test_mean_negative() {
        assert!((mean(&[-3.0, 1.0]) - (-1.0)).abs() < 1e-10);
    }
}
