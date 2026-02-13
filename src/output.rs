use anyhow::Result;

use crate::stats::FeedStats;

pub fn print_pretty(stats: &FeedStats) {
    println!("{:#?}", stats);
}

pub fn print_json(stats: &FeedStats) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(stats)?);
    Ok(())
}
