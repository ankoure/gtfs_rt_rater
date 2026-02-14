use gtfs_rt_rater::parser::parse_feed;
use gtfs_rt_rater::stats::FeedStats;

#[test]
fn test_full_pipeline() {
    // If you have a sample feed file
    let bytes = include_bytes!("fixtures/sample_mbta.pb");
    let feed = parse_feed(bytes).expect("Failed to parse feed");
    let stats = FeedStats::from_feed(&feed);

    assert!(stats.total_entities > 0);
}
