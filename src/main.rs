use anyhow::Result;

use gtfs_rt_rater::{
    parser::parse_feed,
    stats::FeedStats,
    fetch::fetch_bytes,
    output::{print_pretty, print_json},
};

fn main() -> Result<()> {
    let arg = std::env::args().nth(1).expect("provide file or url");

    let bytes = if arg.starts_with("http") {
        fetch_bytes(&arg)?
    } else {
        std::fs::read(arg)?
    };

    let feed = parse_feed(&bytes)?;
    let stats = FeedStats::from_feed(&feed);

    print_pretty(&stats);
    // or:
    // print_json(&stats)?;

    Ok(())
}