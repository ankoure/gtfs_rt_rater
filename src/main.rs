//! CLI entry point for the GTFS-RT Rater tool.
//!
//! Provides subcommands for analyzing individual feeds, consuming all public
//! feeds from MobilityData, aggregating results, and uploading to S3.

mod infra;
mod services;

use crate::infra::mobilitydata::client::MobilityDataClient;
use crate::services::catalog_api::CatalogApi;
use anyhow::Result;
use aws_sdk_s3::primitives::ByteStream;
use chrono::Utc;
use clap::{Parser, Subcommand};
use flate2::Compression;
use flate2::write::GzEncoder;
use gtfs_rt_rater::analyzers::analyzer::{analyze, analyze_for_date};
use gtfs_rt_rater::{
    fetch::{BasicClient, fetch_bytes},
    output::append_record,
    parser::parse_feed,
    stats::FeedStats,
};
use std::ffi::OsStr;
use std::io::Write;
use std::path::Path;
use tracing::Instrument;
use tracing::{debug, error, info, warn};
use tracing_subscriber::{
    EnvFilter, Layer,
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

#[derive(Parser)]
#[command(name = "gtfs_rt_rater")]
#[command(about = "A tool to analyze GTFS-RT feeds", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Analyze a GTFS-RT feed from a file or URL
    Analyze {
        /// Path to file or URL to fetch
        #[arg(value_name = "FILE_OR_URL")]
        source: String,

        /// CSV file to append results to
        #[arg(short, long, default_value = "data.csv")]
        output: String,
    },
    /// Aggregate all feed CSVs and upload results to S3
    Aggregate {
        /// Directory containing CSVs to aggregate
        #[arg(short = 'd', long, default_value = "feeds")]
        output_dir: String,

        /// S3 bucket name to upload aggregated JSON to (e.g., "my-bucket")
        #[arg(long)]
        s3_bucket: String,
    },
    /// List available feeds from MobilityData
    ListFeeds {
        /// Only show vehicle position feeds
        #[arg(short, long, default_value_t = true)]
        vehicle_positions: bool,
    },
    /// Consume all feeds from MobilityData that don't require authentication
    ConsumeAllFeeds {
        /// Directory to save CSV files (one per feed)
        #[arg(short, long, default_value = "feeds")]
        output_dir: String,

        /// Maximum number of concurrent feed downloads
        #[arg(short, long, default_value_t = 5)]
        concurrency: usize,

        /// Sample rate: query each feed every X seconds
        #[arg(short = 'r', long, default_value_t = 60)]
        sample_rate: u64,

        /// Number of samples to collect (0 = infinite)
        #[arg(short = 'n', long, default_value_t = 1)]
        num_samples: usize,

        /// Optional: S3 bucket name to upload files to (e.g., "my-bucket")
        #[arg(long)]
        s3_bucket: Option<String>,

        /// Optional: Gzip compress CSV files before uploading to S3
        #[arg(long, default_value_t = false)]
        gzip: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok(); // Load .env file

    // Logging setup: colored stderr + JSON rolling log file
    let log_file_path =
        std::env::var("LOG_FILE_PATH").unwrap_or_else(|_| "logs/gtfs_rt_rater.log".to_string());
    let log_dir = Path::new(&log_file_path)
        .parent()
        .unwrap_or(Path::new("logs"));
    let log_file_name = Path::new(&log_file_path)
        .file_name()
        .unwrap_or(OsStr::new("gtfs_rt_rater.log"));

    let file_appender = tracing_appender::rolling::daily(log_dir, log_file_name);
    let (non_blocking_file, _file_guard) = tracing_appender::non_blocking(file_appender);

    let stderr_layer = fmt::layer()
        .with_target(true)
        .with_span_events(FmtSpan::CLOSE)
        .with_ansi(true)
        .with_writer(std::io::stderr)
        .with_filter(EnvFilter::from_env("RUST_LOG").add_directive("info".parse().unwrap()));

    let json_layer = fmt::layer()
        .json()
        .with_current_span(true)
        .with_span_list(true)
        .with_writer(non_blocking_file)
        .with_filter(EnvFilter::from_env("RUST_LOG_JSON").add_directive("debug".parse().unwrap()));

    tracing_subscriber::registry()
        .with(stderr_layer)
        .with(json_layer)
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Analyze { source, output } => {
            let bytes = fetcher(&source).await?;
            let feed = parse_feed(&bytes)?;
            let stats = FeedStats::from_feed(&feed);

            append_record(&output, &stats)?;
        }
        Commands::Aggregate {
            output_dir,
            s3_bucket,
        } => {
            if s3_bucket.is_empty() {
                info!("S3 bucket not specified, skipping upload");
            } else {
                analyze(&s3_bucket, &output_dir).await?;
            }
        }
        Commands::ListFeeds {
            vehicle_positions: _,
        } => {
            let refresh_token = std::env::var("MOBILITYDATA_REFRESH_TOKEN")
                .expect("MOBILITYDATA_REFRESH_TOKEN must be set");
            let client = MobilityDataClient::new(refresh_token).await?;

            let feeds = client.list_feeds().await?;

            info!(total = feeds.len(), "Feed list fetched");

            for feed in &feeds {
                let status_str = feed.status.as_deref().unwrap_or("active");
                let auth_str = if feed.requires_auth {
                    "auth-required"
                } else {
                    "open"
                };
                let has_url = feed.url.is_some();

                info!(
                    feed_id = %feed.id,
                    feed_name = %feed.name,
                    status = status_str,
                    auth = auth_str,
                    has_url,
                    "Feed"
                );
            }

            let deprecated_count = feeds
                .iter()
                .filter(|f| f.status.as_deref() == Some("deprecated"))
                .count();
            let auth_required = feeds.iter().filter(|f| f.requires_auth).count();
            let no_url = feeds.iter().filter(|f| f.url.is_none()).count();

            let processable = feeds
                .iter()
                .filter(|f| {
                    !f.requires_auth && f.url.is_some() && f.status.as_deref() != Some("deprecated")
                })
                .count();

            info!(
                total = feeds.len(),
                deprecated = deprecated_count,
                auth_required,
                no_url,
                processable,
                "Feed list summary"
            );
        }
        Commands::ConsumeAllFeeds {
            output_dir,
            concurrency,
            sample_rate,
            num_samples,
            s3_bucket,
            gzip,
        } => {
            consume_all_feeds(
                &output_dir,
                concurrency,
                sample_rate,
                num_samples,
                s3_bucket,
                gzip,
            )
            .await?;
        }
    }

    Ok(())
}

/// Loads feed data from a local file path or fetches it over HTTP.
#[tracing::instrument(fields(source = %url))]
async fn fetcher(url: &String) -> Result<Vec<u8>> {
    let bytes = if url.starts_with("http") {
        let client = BasicClient::new();
        fetch_bytes(&client, &url).await?
    } else {
        std::fs::read(url)?
    };
    Ok(bytes)
}

/// Fetches all public GTFS-RT feeds concurrently, collecting samples at a
/// configurable interval and optionally uploading previous-day results to S3.
#[tracing::instrument(
    skip(s3_bucket, gzip),
    fields(output_dir, concurrency, sample_rate, num_samples)
)]
async fn consume_all_feeds(
    output_dir: &str,
    concurrency: usize,
    sample_rate: u64,
    num_samples: usize,
    s3_bucket: Option<String>,
    gzip: bool,
) -> Result<()> {
    let refresh_token = std::env::var("MOBILITYDATA_REFRESH_TOKEN")
        .expect("MOBILITYDATA_REFRESH_TOKEN must be set");
    let client = MobilityDataClient::new(refresh_token).await?;

    // Initialize S3 client if bucket is provided
    let s3_client = if s3_bucket.is_some() {
        let config = aws_config::load_from_env().await;
        Some(aws_sdk_s3::Client::new(&config))
    } else {
        None
    };

    if let Some(ref bucket) = s3_bucket {
        info!(bucket = %bucket, gzip, "S3 upload enabled");
    }

    info!("Fetching feed list from MobilityData");
    let feeds = client.list_feeds().await?;

    // Filter feeds that don't require authentication, have a URL, and are not deprecated
    let public_feeds: Vec<_> = feeds
        .into_iter()
        .filter(|f| {
            !f.requires_auth && f.url.is_some() && f.status.as_deref() != Some("deprecated")
        })
        .collect();

    info!(
        feed_count = public_feeds.len(),
        "Public feeds ready for processing"
    );

    if num_samples == 0 {
        info!(sample_rate, "Sampling infinitely. Press Ctrl+C to stop.");
    } else {
        info!(num_samples, sample_rate, "Starting sample collection");
    }

    // Create output directory if it doesn't exist
    std::fs::create_dir_all(output_dir)?;

    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(concurrency));

    let mut sample_count = 0;
    let mut last_upload_date: Option<chrono::NaiveDate> = None;

    loop {
        // Check if we've reached the sample limit (0 = infinite)
        if num_samples > 0 && sample_count >= num_samples {
            break;
        }

        sample_count += 1;

        // Check if we need to upload previous day's files
        let today = Utc::now().date_naive();
        if let Some(ref bucket) = s3_bucket {
            if let Some(s3) = &s3_client {
                // Upload previous day's files if we haven't uploaded today yet
                if last_upload_date.is_none() || last_upload_date.unwrap() < today {
                    if let Some(yesterday) = today.pred_opt() {
                        let s3 = s3.clone();
                        let bucket = bucket.to_string();
                        let output_dir = output_dir.to_string();
                        tokio::spawn(async move {
                            info!(date = %yesterday, "Uploading previous day's files to S3");
                            if let Err(e) = upload_previous_day_files(
                                &s3,
                                &bucket,
                                &output_dir,
                                yesterday,
                                gzip,
                            )
                            .await
                            {
                                error!(error = %e, "Failed to upload previous day's files");
                            } else {
                                info!(date = %yesterday, "Successfully uploaded previous day's files");
                            }

                            info!(date = %yesterday, "Aggregating previous day's data");
                            if let Err(e) =
                                analyze_for_date(&s3, &bucket, &output_dir, yesterday).await
                            {
                                error!(error = %e, "Failed to aggregate previous day's data");
                            } else {
                                info!(date = %yesterday, "Successfully aggregated and cleaned up previous day's data");
                            }
                        });

                        last_upload_date = Some(today);
                    }
                }
            }
        }

        info!(
            sample = sample_count,
            total = if num_samples == 0 {
                None
            } else {
                Some(num_samples)
            },
            "Starting sample round"
        );

        let mut tasks = vec![];

        for feed in &public_feeds {
            let sem = semaphore.clone();
            let output_dir = output_dir.to_string();
            let feed = feed.clone();

            let feed_span = tracing::info_span!(
                "process_feed",
                feed_id = %feed.id,
                feed_name = %feed.name,
            );

            let task = tokio::spawn(
                async move {
                    let _permit = sem.acquire().await.unwrap();

                    let url = feed.url.as_ref().unwrap();

                    let http_client = BasicClient::new();

                    // Create agency directory with date-based CSV files
                    let now = Utc::now();
                    let date = now.format("%Y-%m-%d").to_string();
                    let agency_dir = format!("{}/agency_id={}", output_dir, feed.id);

                    // Create directory structure if it doesn't exist
                    if let Err(e) = std::fs::create_dir_all(&agency_dir) {
                        error!(dir = %agency_dir, error = %e, "Failed to create agency directory");
                        return;
                    }

                    let output_file = format!("{}/date={}.csv", agency_dir, date);

                    let fetch_start = std::time::Instant::now();
                    match fetch_bytes(&http_client, url).await {
                        Ok(bytes) => {
                            let elapsed = fetch_start.elapsed();
                            if elapsed.as_secs() > 15 {
                                warn!(elapsed_secs = elapsed.as_secs(), "Feed fetch was slow");
                            }
                            debug!(bytes = bytes.len(), "Feed bytes received, parsing");
                            match parse_feed(&bytes) {
                                Ok(parsed_feed) => {
                                    debug!(
                                        entity_count = parsed_feed.entity.len(),
                                        "Feed parsed successfully"
                                    );
                                    let stats = FeedStats::from_feed(&parsed_feed)
                                        .with_feed_info(&feed.id, &feed.name);
                                    if let Err(e) = append_record(&output_file, &stats) {
                                        error!(error = %e, "Failed to write stats for feed");
                                    } else {
                                        info!("Feed processed successfully");
                                    }
                                }
                                Err(e) => {
                                    error!(error = %e, "Feed parse failed");
                                    let error_stats =
                                        FeedStats::from_error("parse_error", &e.to_string())
                                            .with_feed_info(&feed.id, &feed.name);
                                    let _ = append_record(&output_file, &error_stats);
                                }
                            }
                        }
                        Err(e) => {
                            error!(error = %e, "Feed HTTP fetch failed");
                            let error_stats = FeedStats::from_error("fetch_error", &e.to_string())
                                .with_feed_info(&feed.id, &feed.name);
                            let _ = append_record(&output_file, &error_stats);
                        }
                    }
                }
                .instrument(feed_span),
            );

            tasks.push(task);
        }

        // Wait for all tasks to complete
        for task in tasks {
            let _ = task.await;
        }

        // If not the last sample, wait before next iteration
        if num_samples == 0 || sample_count < num_samples {
            info!(sample_rate, "Waiting before next sample");
            tokio::time::sleep(tokio::time::Duration::from_secs(sample_rate)).await;
        }
    }

    info!(output_dir, "Finished processing all feeds");
    Ok(())
}

/// Uploads CSV files from the previous day to S3, optionally gzip-compressing them.
#[tracing::instrument(skip(client), fields(bucket, output_dir, date = %date, gzip))]
async fn upload_previous_day_files(
    client: &aws_sdk_s3::Client,
    bucket: &str,
    output_dir: &str,
    date: chrono::NaiveDate,
    gzip: bool,
) -> Result<()> {
    let date_str = date.format("%Y-%m-%d").to_string();
    let target_filename = format!("date={}.csv", date_str);

    // Iterate over agency_id=* directories
    let entries = std::fs::read_dir(output_dir)?;
    let mut upload_count = 0;

    for agency_entry in entries {
        let agency_entry = agency_entry?;
        let agency_path = agency_entry.path();

        // Only process agency_id= directories
        let dir_name = agency_entry.file_name();
        let dir_name_str = dir_name.to_str().unwrap_or("");
        if !agency_path.is_dir() || !dir_name_str.starts_with("agency_id=") {
            continue;
        }

        let feed_id = &dir_name_str["agency_id=".len()..];
        let csv_path = agency_path.join(&target_filename);

        if !csv_path.exists() {
            continue;
        }

        let path = csv_path;
        {
            // Read the file
            let file_contents = std::fs::read(&path)?;

            // Prepare the data to upload
            let (body, s3_key) = if gzip {
                // Gzip compress the file
                let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                encoder.write_all(&file_contents)?;
                let compressed = encoder.finish()?;

                let key = format!("agency_id={}/{}.gz", feed_id, target_filename);
                (compressed, key)
            } else {
                let key = format!("agency_id={}/{}", feed_id, target_filename);
                (file_contents, key)
            };

            // Upload to S3
            client
                .put_object()
                .bucket(bucket)
                .key(&s3_key)
                .body(ByteStream::from(body))
                .send()
                .await?;

            upload_count += 1;
        }
    }

    info!(upload_count, date = %date_str, "S3 upload complete");
    Ok(())
}
