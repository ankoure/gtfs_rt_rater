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
