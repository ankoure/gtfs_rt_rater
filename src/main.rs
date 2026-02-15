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
use gtfs_rt_rater::{
    fetch::{BasicClient, fetch_bytes},
    output::{append_record, print_pretty},
    parser::parse_feed,
    stats::FeedStats,
};
use log::{error, info, warn};
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
    env_logger::init(); // Initialize logger

    let cli = Cli::parse();

    match cli.command {
        Commands::Analyze { source, output } => {
            let bytes = fetcher(&source).await?;
            let feed = parse_feed(&bytes)?;
            let stats = FeedStats::from_feed(&feed);

            print_pretty(&stats);
            append_record(&output, &stats)?;
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
                let auth_str = if feed.requires_auth { "🔒" } else { "🔓" };
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
            let auth_required = feeds.iter().filter(|f| f.requires_auth).count();
            let no_url = feeds.iter().filter(|f| f.url.is_none()).count();

            let processable = feeds
                .iter()
                .filter(|f| {
                    !f.requires_auth && f.url.is_some() && f.status.as_deref() != Some("deprecated")
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

async fn fetcher(url: &String) -> Result<Vec<u8>> {
    let bytes = if url.starts_with("http") {
        let client = BasicClient::new();
        fetch_bytes(&client, &url).await?
    } else {
        std::fs::read(url)?
    };
    Ok(bytes)
}

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
        info!("S3 upload enabled: bucket={}, gzip={}", bucket, gzip);
    }

    info!("Fetching feed list from MobilityData...");
    let feeds = client.list_feeds().await?;

    // Filter feeds that don't require authentication, have a URL, and are not deprecated
    let public_feeds: Vec<_> = feeds
        .into_iter()
        .filter(|f| {
            !f.requires_auth && f.url.is_some() && f.status.as_deref() != Some("deprecated")
        })
        .collect();

    info!(
        "Found {} public feeds to process (excluding deprecated)",
        public_feeds.len()
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
                        info!("\n=== Uploading previous day's files to S3 ===");
                        if let Err(e) =
                            upload_previous_day_files(s3, bucket, output_dir, yesterday, gzip).await
                        {
                            error!("Failed to upload previous day's files: {}", e);
                        } else {
                            info!("✓ Successfully uploaded previous day's files");
                            last_upload_date = Some(today);
                        }
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

            let task = tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();

                let url = feed.url.as_ref().unwrap();

                let http_client = BasicClient::new();

                // Create daily file path based on UTC date
                let now = Utc::now();
                let year = now.format("%Y").to_string();
                let month = now.format("%m").to_string();
                let day = now.format("%d").to_string();
                let daily_dir = format!("{}/Year={}/Month={}/Day={}", output_dir, year, month, day);

                // Create directory structure if it doesn't exist
                if let Err(e) = std::fs::create_dir_all(&daily_dir) {
                    error!("Failed to create directory {}: {}", daily_dir, e);
                    return;
                }

                let output_file = format!("{}/{}.csv", daily_dir, feed.id);

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

async fn upload_previous_day_files(
    client: &aws_sdk_s3::Client,
    bucket: &str,
    output_dir: &str,
    date: chrono::NaiveDate,
    gzip: bool,
) -> Result<()> {
    let year = date.format("%Y").to_string();
    let month = date.format("%m").to_string();
    let day = date.format("%d").to_string();

    let daily_dir = format!("{}/Year={}/Month={}/Day={}", output_dir, year, month, day);

    // Check if directory exists
    if !std::path::Path::new(&daily_dir).exists() {
        warn!(
            "No directory found for {}-{}-{}, skipping upload",
            year, month, day
        );
        return Ok(());
    }

    // Read all CSV files in the directory
    let entries = std::fs::read_dir(&daily_dir)?;
    let mut upload_count = 0;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        // Only process .csv files
        if path.extension().and_then(|s| s.to_str()) == Some("csv") {
            let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");

            // Read the file
            let file_contents = std::fs::read(&path)?;

            // Prepare the data to upload
            let (body, s3_key) = if gzip {
                // Gzip compress the file
                let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                encoder.write_all(&file_contents)?;
                let compressed = encoder.finish()?;

                let key = format!("Year={}/Month={}/Day={}/{}.gz", year, month, day, file_name);
                (compressed, key)
            } else {
                let key = format!("Year={}/Month={}/Day={}/{}", year, month, day, file_name);
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

    info!(
        "✓ Uploaded {} files for {}-{}-{}",
        upload_count, year, month, day
    );
    Ok(())
}
