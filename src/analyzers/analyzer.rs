use crate::analyzers::aggregate::aggregate_feed;
use crate::analyzers::types::{FeedIndex, FeedIndexEntry, FeedStats};
use crate::analyzers::writetos3::write_json_to_s3;
use anyhow::Result;
use chrono::NaiveDate;
use log::info;
use std::fs;
use std::fs::File;

/// Aggregates all local feed CSVs, uploads per-feed JSON and an index to S3,
/// then deletes the processed CSVs.
pub async fn analyze(bucket: &str, base_dir: &str) -> anyhow::Result<()> {
    let config = aws_config::load_from_env().await;
    let s3 = aws_sdk_s3::Client::new(&config);

    let feed_ids = load_feed_ids(base_dir)?;

    let mut index_entries = Vec::new();

    for feed_id in feed_ids {
        // Load local CSVs for feed
        let rows = load_feed_rows(base_dir, &feed_id)?;
        if rows.is_empty() {
            continue;
        }

        // Aggregate
        let aggregate = aggregate_feed(&feed_id, rows)?;

        // Upload JSON to S3
        write_json_to_s3(
            &s3,
            bucket,
            &format!("aggregates/feeds/{}.json", feed_id),
            &aggregate,
        )
        .await?;

        // Add to index
        index_entries.push(FeedIndexEntry {
            feed_id: feed_id.to_string(),
            overall_grade: aggregate.overall.grade.clone(),
            overall_score: aggregate.overall.score,
            uptime_percent: aggregate.entity_stats.uptime_percent,
        });

        // Delete local CSVs
        delete_feed_csvs(base_dir, &feed_id)?;
    }

    // Write homepage index JSON
    let index = FeedIndex {
        generated_at: chrono::Utc::now(),
        feeds: index_entries,
    };
    write_json_to_s3(&s3, bucket, "aggregates/feeds.json", &index).await?;

    Ok(())
}

fn load_feed_ids(base_dir: &str) -> Result<Vec<String>> {
    let mut feed_ids = Vec::new();

    for entry in fs::read_dir(base_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            if let Some(dir_name) = entry.file_name().to_str() {
                if let Some(feed_id) = dir_name.strip_prefix("agency_id=") {
                    feed_ids.push(feed_id.to_string());
                }
            }
        }
    }

    Ok(feed_ids)
}

fn load_feed_rows(base_dir: &str, feed_id: &str) -> Result<Vec<FeedStats>> {
    let mut rows = Vec::new();
    let feed_dir = format!("{}/agency_id={}", base_dir, feed_id);

    for entry in fs::read_dir(&feed_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) != Some("csv") {
            continue;
        }

        let file = File::open(path)?;
        let mut rdr = csv::Reader::from_reader(file);

        for result in rdr.deserialize() {
            let record: FeedStats = result?;
            rows.push(record);
        }
    }

    Ok(rows)
}

fn delete_feed_csvs(base_dir: &str, feed_id: &str) -> Result<()> {
    let feed_dir = format!("{}/agency_id={}", base_dir, feed_id);

    for entry in fs::read_dir(&feed_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("csv") {
            fs::remove_file(path)?;
        }
    }

    Ok(())
}

/// Analyze and aggregate feeds for a specific date, upload JSON to S3, then delete local CSVs.
pub async fn analyze_for_date(
    s3: &aws_sdk_s3::Client,
    bucket: &str,
    base_dir: &str,
    date: NaiveDate,
) -> Result<()> {
    let date_str = date.format("%Y-%m-%d").to_string();
    info!("Starting aggregation for date {}", date_str);

    let feed_ids = load_feed_ids(base_dir)?;
    let mut index_entries = Vec::new();

    for feed_id in feed_ids {
        let rows = load_feed_rows_for_date(base_dir, &feed_id, &date_str)?;
        if rows.is_empty() {
            continue;
        }

        let aggregate = aggregate_feed(&feed_id, rows)?;

        write_json_to_s3(
            s3,
            bucket,
            &format!("aggregates/feeds/{}.json", feed_id),
            &aggregate,
        )
        .await?;

        index_entries.push(FeedIndexEntry {
            feed_id: feed_id.to_string(),
            overall_grade: aggregate.overall.grade.clone(),
            overall_score: aggregate.overall.score,
            uptime_percent: aggregate.entity_stats.uptime_percent,
        });

        delete_feed_csv_for_date(base_dir, &feed_id, &date_str)?;
    }

    let index = FeedIndex {
        generated_at: chrono::Utc::now(),
        feeds: index_entries,
    };
    write_json_to_s3(s3, bucket, "aggregates/feeds.json", &index).await?;

    info!("Aggregation complete for date {}", date_str);
    Ok(())
}

fn load_feed_rows_for_date(base_dir: &str, feed_id: &str, date_str: &str) -> Result<Vec<FeedStats>> {
    let csv_path = format!("{}/agency_id={}/date={}.csv", base_dir, feed_id, date_str);
    let path = std::path::Path::new(&csv_path);

    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = File::open(path)?;
    let mut rdr = csv::Reader::from_reader(file);
    let mut rows = Vec::new();

    for result in rdr.deserialize() {
        let record: FeedStats = result?;
        rows.push(record);
    }

    Ok(rows)
}

fn delete_feed_csv_for_date(base_dir: &str, feed_id: &str, date_str: &str) -> Result<()> {
    let csv_path = format!("{}/agency_id={}/date={}.csv", base_dir, feed_id, date_str);
    let path = std::path::Path::new(&csv_path);

    if path.exists() {
        fs::remove_file(path)?;
        info!("Deleted {}", csv_path);
    }

    Ok(())
}
