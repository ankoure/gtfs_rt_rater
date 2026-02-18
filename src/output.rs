//! Output formatting and persistence for feed statistics.
//!
//! Supports pretty-printing, JSON serialization, and CSV append.

use anyhow::Result;
use log::info;

use crate::stats::FeedStats;
use csv::WriterBuilder;
use std::fs::OpenOptions;
use std::path::Path;

/// Logs feed statistics using Rust's debug pretty-print format.
pub fn print_pretty(stats: &FeedStats) {
    info!("{:#?}", stats);
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

    let file = OpenOptions::new().append(true).create(true).open(path)?;

    let mut writer = WriterBuilder::new()
        .has_headers(!file_exists) // IMPORTANT when appending
        .from_writer(file);

    writer.serialize(feed_stats)?;
    writer.flush()?;

    Ok(())
}
