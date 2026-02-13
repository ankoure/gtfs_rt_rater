use anyhow::Result;

use gtfs_rt_rater::{
    parser::parse_feed,
    stats::FeedStats,
    fetch::{fetch_bytes, BasicClient},
    output::{print_pretty, print_json},
};

#[tokio::main]
async fn main() -> Result<()> {
    let arg = std::env::args().nth(1).expect("provide file or url");

    let bytes = if arg.starts_with("http") {
        let client = BasicClient::new();
        fetch_bytes(&client, &arg).await?
    } else {
        std::fs::read(arg)?
    };

    let feed = parse_feed(&bytes)?;
    let stats = FeedStats::from_feed(&feed);

    print_pretty(&stats);

    Ok(())
}
