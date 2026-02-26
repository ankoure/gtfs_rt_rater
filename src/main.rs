//! CLI entry point for the GTFS-RT Rater tool.
//!
//! Provides subcommands for analyzing individual feeds, consuming all public
//! feeds from MobilityData, aggregating results, and uploading to S3.

mod infra;
mod services;

use crate::infra::keys::{FeedKeyConfig, KeyStore, SsmKeyStore};
use crate::infra::mobilitydata::client::MobilityDataClient;
use crate::services::catalog_api::{CatalogApi, FeedAuth};
use anyhow::Result;
use aws_sdk_s3::primitives::ByteStream;
use chrono::Utc;
use clap::{Parser, Subcommand};
use flate2::Compression;
use flate2::write::GzEncoder;
use gtfs_rt_rater::analyzers::analyzer::{analyze, analyze_for_date};
use gtfs_rt_rater::{
    fetch::{
        BasicClient, HttpClient,
        auth::{api_key::ApiKey, url_param::UrlParam},
        fetch_bytes,
    },
    output::append_record,
    parser::parse_feed,
    stats::FeedStats,
};
use log::{error, info};
use std::io::Write;

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

        /// Optional: path to a JSON file mapping feed IDs to SSM parameter paths
        /// (e.g. {"mdb-123": "/gtfs/feeds/mdb-123/api_key"}).
        /// When provided, authenticated feeds with a matching entry will be
        /// included in the run.
        #[arg(long)]
        key_config: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok(); // Load .env file
    env_logger::init(); // Initialize logger

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
                info!("S3 bucket is empty, skipping upload");
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

            info!("Total feeds: {}\n", feeds.len());

            for feed in &feeds {
                let status_str = feed.status.as_deref().unwrap_or("active");
                let auth_str = if feed.auth.requires_auth() {
                    "🔒"
                } else {
                    "🔓"
                };
                let url_str = if feed.url.is_some() { "✓" } else { "✗" };

                info!(
                    "{} {} [{}] {} - {}",
                    auth_str, url_str, status_str, feed.id, feed.name
                );
            }

            let deprecated_count = feeds
                .iter()
                .filter(|f| f.status.as_deref() == Some("deprecated"))
                .count();
            let auth_required = feeds.iter().filter(|f| f.auth.requires_auth()).count();
            let no_url = feeds.iter().filter(|f| f.url.is_none()).count();

            let processable = feeds
                .iter()
                .filter(|f| {
                    !f.auth.requires_auth()
                        && f.url.is_some()
                        && f.status.as_deref() != Some("deprecated")
                })
                .count();

            info!("\nSummary:");
            info!("  Total feeds: {}", feeds.len());
            info!("  Deprecated: {}", deprecated_count);
            info!("  Auth required: {}", auth_required);
            info!("  No URL: {}", no_url);
            info!("  Processable by consume-all-feeds: {}", processable);
        }
        Commands::ConsumeAllFeeds {
            output_dir,
            concurrency,
            sample_rate,
            num_samples,
            s3_bucket,
            gzip,
            key_config,
        } => {
            consume_all_feeds(
                &output_dir,
                concurrency,
                sample_rate,
                num_samples,
                s3_bucket,
                gzip,
                key_config.as_deref(),
            )
            .await?;
        }
    }

    Ok(())
}

/// Loads feed data from a local file path or fetches it over HTTP.
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
///
/// If `key_config_path` is provided, feeds that require authentication are
/// also included when a matching entry exists in the config file. Keys are
/// resolved from SSM once at startup and cached for the duration of the run.
async fn consume_all_feeds(
    output_dir: &str,
    concurrency: usize,
    sample_rate: u64,
    num_samples: usize,
    s3_bucket: Option<String>,
    gzip: bool,
    key_config_path: Option<&str>,
) -> Result<()> {
    let refresh_token = std::env::var("MOBILITYDATA_REFRESH_TOKEN")
        .expect("MOBILITYDATA_REFRESH_TOKEN must be set");
    let client = MobilityDataClient::new(refresh_token).await?;

    // Load AWS config once; reused for both S3 and SSM clients.
    let aws_config = aws_config::load_from_env().await;

    // Initialize S3 client if bucket is provided
    let s3_client = if s3_bucket.is_some() {
        Some(aws_sdk_s3::Client::new(&aws_config))
    } else {
        None
    };

    if let Some(ref bucket) = s3_bucket {
        info!("S3 upload enabled: bucket={}, gzip={}", bucket, gzip);
    }

    info!("Fetching feed list from MobilityData...");
    let feeds = client.list_feeds().await?;

    // Optionally load the key config and resolve API keys from SSM upfront.
    // Wrapped in Arc so spawned tasks can share without cloning the full map.
    // The resolved map is keyed by feed_id and contains the plaintext API key.
    let resolved_keys: std::sync::Arc<std::collections::HashMap<String, String>> =
        std::sync::Arc::new(if let Some(path) = key_config_path {
            let key_config = FeedKeyConfig::load(path)?;
            let store = SsmKeyStore::new(&aws_config);

            let mut map = std::collections::HashMap::new();
            for (feed_id, reference) in key_config.iter() {
                match store.get(reference).await {
                    Ok(key) => {
                        info!("✓ Resolved key for {feed_id} from SSM ({reference})");
                        map.insert(feed_id.to_string(), key);
                    }
                    Err(e) => {
                        error!("✗ Failed to resolve key for {feed_id} ({reference}): {e}");
                    }
                }
            }
            map
        } else {
            std::collections::HashMap::new()
        });

    // Include public feeds and any authenticated feeds for which we have a key.
    let active_feeds: Vec<_> = feeds
        .into_iter()
        .filter(|f| {
            f.url.is_some()
                && f.status.as_deref() != Some("deprecated")
                && match &f.auth {
                    FeedAuth::None => true,
                    _ => resolved_keys.contains_key(&f.id),
                }
        })
        .collect();

    let auth_count = active_feeds
        .iter()
        .filter(|f| f.auth != FeedAuth::None)
        .count();
    let public_feeds = active_feeds; // rename for the rest of the function

    info!(
        "Found {} feeds to process ({} authenticated, excluding deprecated)",
        public_feeds.len(),
        auth_count,
    );

    if num_samples == 0 {
        info!(
            "Sampling infinitely every {} seconds. Press Ctrl+C to stop.",
            sample_rate
        );
    } else {
        info!(
            "Collecting {} sample(s) every {} seconds",
            num_samples, sample_rate
        );
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
                            info!("\n=== Uploading previous day's files to S3 ===");
                            if let Err(e) = upload_previous_day_files(
                                &s3,
                                &bucket,
                                &output_dir,
                                yesterday,
                                gzip,
                            )
                            .await
                            {
                                error!("Failed to upload previous day's files: {}", e);
                            } else {
                                info!("✓ Successfully uploaded previous day's files");
                            }

                            info!("\n=== Aggregating previous day's data ===");
                            if let Err(e) =
                                analyze_for_date(&s3, &bucket, &output_dir, yesterday).await
                            {
                                error!("Failed to aggregate previous day's data: {}", e);
                            } else {
                                info!(
                                    "✓ Successfully aggregated and cleaned up previous day's data"
                                );
                            }
                        });

                        last_upload_date = Some(today);
                    }
                }
            }
        }

        info!(
            "\n=== Sample {} {} ===",
            sample_count,
            if num_samples == 0 {
                "(infinite mode)".to_string()
            } else {
                format!("of {}", num_samples)
            }
        );

        let mut tasks = vec![];

        for feed in &public_feeds {
            let sem = semaphore.clone();
            let output_dir = output_dir.to_string();
            let feed = feed.clone();

            let resolved_keys = resolved_keys.clone();
            let task = tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();

                let url = feed.url.as_ref().unwrap();

                // Build the appropriate HTTP client for this feed's auth type.
                let http_client: Box<dyn HttpClient> = match &feed.auth {
                    FeedAuth::None => Box::new(BasicClient::new()),
                    FeedAuth::Header { header_name } => {
                        let key = resolved_keys[&feed.id].clone();
                        Box::new(ApiKey {
                            inner: BasicClient::new(),
                            header_name: header_name.clone(),
                            key,
                        })
                    }
                    FeedAuth::UrlParam { param_name } => {
                        let key = resolved_keys[&feed.id].clone();
                        Box::new(UrlParam {
                            inner: BasicClient::new(),
                            param_name: param_name.clone(),
                            key,
                        })
                    }
                };

                // Create agency directory with date-based CSV files
                let now = Utc::now();
                let date = now.format("%Y-%m-%d").to_string();
                let agency_dir = format!("{}/agency_id={}", output_dir, feed.id);

                // Create directory structure if it doesn't exist
                if let Err(e) = std::fs::create_dir_all(&agency_dir) {
                    error!("Failed to create directory {}: {}", agency_dir, e);
                    return;
                }

                let output_file = format!("{}/date={}.csv", agency_dir, date);

                match fetch_bytes(&http_client, url).await {
                    Ok(bytes) => match parse_feed(&bytes) {
                        Ok(parsed_feed) => {
                            let stats = FeedStats::from_feed(&parsed_feed)
                                .with_feed_info(&feed.id, &feed.name);
                            if let Err(e) = append_record(&output_file, &stats) {
                                error!("Failed to write stats for {}: {}", feed.id, e);
                            } else {
                                info!("✓ {} - {}", feed.id, feed.name);
                            }
                        }
                        Err(e) => {
                            error!("✗ Failed to parse feed {}: {}", feed.id, e);
                            let error_stats = FeedStats::from_error("parse_error", &e.to_string())
                                .with_feed_info(&feed.id, &feed.name);
                            let _ = append_record(&output_file, &error_stats);
                        }
                    },
                    Err(e) => {
                        error!("✗ Failed to fetch feed {}: {}", feed.id, e);
                        let error_stats = FeedStats::from_error("fetch_error", &e.to_string())
                            .with_feed_info(&feed.id, &feed.name);
                        let _ = append_record(&output_file, &error_stats);
                    }
                }
            });

            tasks.push(task);
        }

        // Wait for all tasks to complete
        for task in tasks {
            let _ = task.await;
        }

        // If not the last sample, wait before next iteration
        if num_samples == 0 || sample_count < num_samples {
            info!("Waiting {} seconds until next sample...", sample_rate);
            tokio::time::sleep(tokio::time::Duration::from_secs(sample_rate)).await;
        }
    }

    info!(
        "\nFinished processing all feeds. Results saved to {}/",
        output_dir
    );
    Ok(())
}

/// Uploads CSV files from the previous day to S3, optionally gzip-compressing them.
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

    info!("✓ Uploaded {} files for {}", upload_count, date_str);
    Ok(())
}
