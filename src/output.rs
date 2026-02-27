//! Output formatting and persistence for feed statistics.
//!
//! Supports pretty-printing, JSON serialization, and CSV append.

use anyhow::Result;
use tracing::{debug, info};

use crate::stats::FeedStats;
use csv::WriterBuilder;
use std::fs::OpenOptions;
use std::path::Path;

/// Logs feed statistics using Rust's debug pretty-print format.
pub fn print_pretty(stats: &FeedStats) {
    debug!("{:#?}", stats);
}

/// Logs feed statistics as pretty-printed JSON.
pub fn print_json(stats: &FeedStats) -> Result<()> {
    info!("{}", serde_json::to_string_pretty(stats)?);
    Ok(())
}

/// Appends a [`FeedStats`] record as a row to a CSV file.
///
/// Creates the file with headers if it does not already exist.
pub fn append_record(path: &str, feed_stats: &FeedStats) -> Result<()> {
    let file_exists = Path::new(path).exists();
    debug!(path, file_exists, "Appending CSV record");

    let file = OpenOptions::new().append(true).create(true).open(path)?;

    let mut writer = WriterBuilder::new()
        .has_headers(!file_exists) // IMPORTANT when appending
        .from_writer(file);

    writer.serialize(feed_stats)?;
    writer.flush()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::FeedStats;
    use std::env;
    use std::fs;

    fn temp_path(name: &str) -> String {
        format!("{}/{}", env::temp_dir().display(), name)
    }

    #[test]
    fn test_print_pretty_does_not_panic() {
        let stats = FeedStats::default();
        print_pretty(&stats);
    }

    #[test]
    fn test_print_json_does_not_panic() {
        let stats = FeedStats::default();
        print_json(&stats).unwrap();
    }

    #[test]
    fn test_append_record_creates_file() {
        let path = temp_path("gtfs_rt_rater_test_create.csv");
        let _ = fs::remove_file(&path); // clean up any prior run

        let stats = FeedStats::default();
        append_record(&path, &stats).unwrap();

        assert!(Path::new(&path).exists());
        let content = fs::read_to_string(&path).unwrap();
        assert!(!content.is_empty());

        fs::remove_file(&path).unwrap();
    }

    #[test]
    fn test_append_record_writes_header_once() {
        let path = temp_path("gtfs_rt_rater_test_header.csv");
        let _ = fs::remove_file(&path);

        let stats = FeedStats::default();
        append_record(&path, &stats).unwrap();
        append_record(&path, &stats).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        // Header line should appear exactly once
        let header_count = content.lines().filter(|l| l.contains("timestamp")).count();
        assert_eq!(header_count, 1);

        fs::remove_file(&path).unwrap();
    }

    #[test]
    fn test_append_record_two_rows() {
        let path = temp_path("gtfs_rt_rater_test_rows.csv");
        let _ = fs::remove_file(&path);

        let stats = FeedStats::default();
        append_record(&path, &stats).unwrap();
        append_record(&path, &stats).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        // 1 header + 2 data rows = 3 lines (last may be empty due to trailing newline)
        let lines: Vec<_> = content.lines().collect();
        assert_eq!(lines.len(), 3);

        fs::remove_file(&path).unwrap();
    }
}
