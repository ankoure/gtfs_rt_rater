mod infra;
mod services;

use crate::infra::mobilitydata::client::MobilityDataClient;
use crate::services::catalog_api::CatalogApi;
use anyhow::Result;
use clap::{Parser, Subcommand};
use gtfs_rt_rater::{
    fetch::{BasicClient, fetch_bytes},
    output::{append_record, print_pretty},
    parser::parse_feed,
    stats::FeedStats,
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
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok(); // Load .env file

    let cli = Cli::parse();

    match cli.command {
        Commands::Analyze { source, output } => {
            let bytes = fetcher(&source).await?;
            let feed = parse_feed(&bytes)?;
            let stats = FeedStats::from_feed(&feed);

            print_pretty(&stats);
            append_record(&output, &stats)?;
        }
        Commands::ListFeeds { vehicle_positions: _ } => {
            let refresh_token = std::env::var("MOBILITYDATA_REFRESH_TOKEN")
                .expect("MOBILITYDATA_REFRESH_TOKEN must be set");
            let client = MobilityDataClient::new(refresh_token).await?;

            let feeds = client.list_feeds().await?;

            println!("Total feeds: {}\n", feeds.len());

            for feed in &feeds {
                let status_str = feed.status.as_deref().unwrap_or("active");
                let auth_str = if feed.requires_auth { "🔒" } else { "🔓" };
                let url_str = if feed.url.is_some() { "✓" } else { "✗" };

                println!("{} {} [{}] {} - {}",
                    auth_str, url_str, status_str, feed.id, feed.name);
            }

            let deprecated_count = feeds.iter().filter(|f| f.status.as_deref() == Some("deprecated")).count();
            let auth_required = feeds.iter().filter(|f| f.requires_auth).count();
            let no_url = feeds.iter().filter(|f| f.url.is_none()).count();

            let processable = feeds.iter().filter(|f| {
                !f.requires_auth
                && f.url.is_some()
                && f.status.as_deref() != Some("deprecated")
            }).count();

            println!("\nSummary:");
            println!("  Total feeds: {}", feeds.len());
            println!("  Deprecated: {}", deprecated_count);
            println!("  Auth required: {}", auth_required);
            println!("  No URL: {}", no_url);
            println!("  Processable by consume-all-feeds: {}", processable);
        }
        Commands::ConsumeAllFeeds { output_dir, concurrency, sample_rate, num_samples } => {
            consume_all_feeds(&output_dir, concurrency, sample_rate, num_samples).await?;
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

async fn consume_all_feeds(output_dir: &str, concurrency: usize, sample_rate: u64, num_samples: usize) -> Result<()> {
    let refresh_token = std::env::var("MOBILITYDATA_REFRESH_TOKEN")
        .expect("MOBILITYDATA_REFRESH_TOKEN must be set");
    let client = MobilityDataClient::new(refresh_token).await?;

    println!("Fetching feed list from MobilityData...");
    let feeds = client.list_feeds().await?;

    // Filter feeds that don't require authentication, have a URL, and are not deprecated
    let public_feeds: Vec<_> = feeds
        .into_iter()
        .filter(|f| {
            !f.requires_auth
            && f.url.is_some()
            && f.status.as_deref() != Some("deprecated")
        })
        .collect();

    println!("Found {} public feeds to process (excluding deprecated)", public_feeds.len());

    if num_samples == 0 {
        println!("Sampling infinitely every {} seconds. Press Ctrl+C to stop.", sample_rate);
    } else {
        println!("Collecting {} sample(s) every {} seconds", num_samples, sample_rate);
    }

    // Create output directory if it doesn't exist
    std::fs::create_dir_all(output_dir)?;

    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(concurrency));

    let mut sample_count = 0;
    loop {
        // Check if we've reached the sample limit (0 = infinite)
        if num_samples > 0 && sample_count >= num_samples {
            break;
        }

        sample_count += 1;
        println!("\n=== Sample {} {} ===", sample_count, if num_samples == 0 { "(infinite mode)".to_string() } else { format!("of {}", num_samples) });

        let mut tasks = vec![];

        for feed in &public_feeds {
            let sem = semaphore.clone();
            let output_dir = output_dir.to_string();
            let feed = feed.clone();

            let task = tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();

                let url = feed.url.as_ref().unwrap();

                let http_client = BasicClient::new();
                let output_file = format!("{}/{}.csv", output_dir, feed.id);

                match fetch_bytes(&http_client, url).await {
                    Ok(bytes) => {
                        match parse_feed(&bytes) {
                            Ok(parsed_feed) => {
                                let stats = FeedStats::from_feed(&parsed_feed)
                                    .with_feed_info(&feed.id, &feed.name);
                                if let Err(e) = append_record(&output_file, &stats) {
                                    eprintln!("Failed to write stats for {}: {}", feed.id, e);
                                } else {
                                    println!("✓ {} - {}", feed.id, feed.name);
                                }
                            }
                            Err(e) => {
                                eprintln!("✗ Failed to parse feed {}: {}", feed.id, e);
                                let error_stats = FeedStats::from_error("parse_error", &e.to_string())
                                    .with_feed_info(&feed.id, &feed.name);
                                let _ = append_record(&output_file, &error_stats);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("✗ Failed to fetch feed {}: {}", feed.id, e);
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
            println!("Waiting {} seconds until next sample...", sample_rate);
            tokio::time::sleep(tokio::time::Duration::from_secs(sample_rate)).await;
        }
    }

    println!("\nFinished processing all feeds. Results saved to {}/", output_dir);
    Ok(())
}
