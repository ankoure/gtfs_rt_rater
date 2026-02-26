/// Converts a support proportion (0.0–1.0) into a letter grade.
///
/// | Range       | Grade |
/// |-------------|-------|
/// | >= 0.95     | A+    |
/// | >= 0.90     | A     |
/// | >= 0.80     | B     |
/// | >= 0.65     | C     |
/// | >= 0.40     | D     |
/// | < 0.40      | F     |
pub fn grade(p: f64) -> String {
    match p {
        p if p >= 0.95 => "A+".into(),
        p if p >= 0.90 => "A".into(),
        p if p >= 0.80 => "B".into(),
        p if p >= 0.65 => "C".into(),
        p if p >= 0.40 => "D".into(),
        _ => "F".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grade_boundaries() {
        assert_eq!(grade(1.00), "A+");
        assert_eq!(grade(0.95), "A+");
        assert_eq!(grade(0.94), "A");
        assert_eq!(grade(0.90), "A");
        assert_eq!(grade(0.89), "B");
        assert_eq!(grade(0.80), "B");
        assert_eq!(grade(0.79), "C");
        assert_eq!(grade(0.65), "C");
        assert_eq!(grade(0.64), "D");
        assert_eq!(grade(0.40), "D");
        assert_eq!(grade(0.39), "F");
        assert_eq!(grade(0.00), "F");
    }
}
