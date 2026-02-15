# GTFS Realtime Rater

A Rust CLI tool for parsing and analyzing GTFS Realtime feeds. It calculates statistics about the completeness and quality of transit feed data.

## Features

- **Parse GTFS Realtime feeds** from files or URLs
- **Calculate statistics** on feed entities (vehicles, trip updates, alerts, etc.)
- **Analyze field coverage** for vehicle positions (bearing, speed, occupancy, etc.)
- **MobilityData integration** - automatically discover and process feeds from the MobilityData Catalog
- **Batch processing** - consume all public feeds concurrently
- **Time-series sampling** - collect samples at regular intervals for monitoring feed health over time
- **Multiple output formats** - pretty-print, JSON, and CSV
- **Configurable timeouts** - 30s request timeout, 10s connect timeout

## Requirements

- Rust (edition 2024)

## Installation
```bash
cargo build --release
```
## Usage

The tool provides several commands for analyzing GTFS-RT feeds:

### Analyze a Single Feed

Analyze a local GTFS-RT feed file:

```bash
cargo run -- analyze path/to/feed.pb
```

Analyze a GTFS-RT feed from a URL:

```bash
cargo run -- analyze https://cdn.mbta.com/realtime/VehiclePositions.pb
```

Specify a custom output CSV file:

```bash
cargo run -- analyze https://example.com/feed.pb --output custom-output.csv
```

### List Available Feeds

List all vehicle position feeds from MobilityData with status information:

```bash
cargo run -- list-feeds
```

Output shows:
- 🔓 = No authentication required, 🔒 = Authentication required
- ✓ = Has URL, ✗ = No URL
- Status: active, deprecated, inactive, development, or future
- Summary statistics (deprecated count, auth required, etc.)

### Consume All Public Feeds

Automatically fetch and analyze all public vehicle position feeds from MobilityData.

**Note:** This command automatically filters out:
- Feeds requiring authentication
- Feeds without URLs
- Deprecated feeds

**Basic usage** (single sample):
```bash
cargo run -- consume-all-feeds
```

**Collect multiple samples** (10 samples, every 30 seconds):
```bash
cargo run -- consume-all-feeds -r 30 -n 10
```

**Run continuously** (infinite sampling every 2 minutes):
```bash
cargo run -- consume-all-feeds -r 120 -n 0
```

**High-frequency monitoring** (sample every 10 seconds with 10 concurrent downloads):
```bash
cargo run -- consume-all-feeds -c 10 -r 10 -n 0
```

**Custom output directory**:
```bash
cargo run -- consume-all-feeds --output-dir my-feeds -r 60 -n 5
```

**Upload to S3 with gzip compression**:
```bash
cargo run -- consume-all-feeds --s3-bucket my-bucket --gzip -r 60 -n 0
```

**How S3 uploads work:**
- CSV files are written locally to `feeds/Year=2026/Month=02/Day=15/{agency-id}.csv` based on the current UTC date
- Each day at first sample, the previous day's completed files are automatically gzipped and uploaded to S3
- S3 path pattern: `Year={year}/Month={month}/Day={day}/{agency-id}.csv.gz`
- This ensures complete daily files are uploaded without repeatedly uploading growing files

#### Options for `consume-all-feeds`:

- `-o, --output-dir <DIR>` - Directory to save CSV files (one per feed, default: `feeds/`)
- `-c, --concurrency <N>` - Maximum concurrent downloads (default: 5)
- `-r, --sample-rate <SEC>` - Query each feed every X seconds (default: 60)
- `-n, --num-samples <N>` - Number of samples to collect, 0 = infinite (default: 1)
- `--s3-bucket <BUCKET>` - Optional S3 bucket name to upload CSV files (e.g., `my-bucket`)
- `--gzip` - Optional flag to gzip compress CSV files before uploading to S3

**Note:** You need to set the `MOBILITYDATA_REFRESH_TOKEN` environment variable in a `.env` file to use MobilityData features.

**Note:** When using S3 upload, ensure your AWS credentials are configured (via environment variables, AWS config files, or IAM roles).


## Output

The tool outputs statistics including:

- **Entity counts**: vehicles, trip updates, alerts, shapes, stops, trip modifications
- **Vehicle field coverage**: position, bearing, speed, odometer, occupancy, timestamps, etc.
- **Error tracking**: Records fetch errors and parse errors with timestamps and details
- **Feed metadata**: Feed ID and provider name for easy identification

### CSV Output Format

Each CSV file contains one row per sample with the following columns:

**Success rows:**
- `timestamp` - When the sample was collected
- `feed_id` - MobilityData feed identifier (e.g., "mdb-2335")
- `feed_name` - Provider name
- `total_entities` - Number of entities in the feed
- Statistics fields (vehicles, with_bearing, etc.)
- `error_type` - Empty for successful fetches
- `error_message` - Empty for successful fetches

**Error rows:**
- Same timestamp, feed_id, and feed_name
- All statistics set to 0
- `error_type` - Either "fetch_error" (network/timeout) or "parse_error" (invalid data)
- `error_message` - Detailed error description

This allows you to track feed reliability over time and identify problematic feeds.

### Console Output Example

```
FeedStats {
    timestamp: 2026-02-14T03:08:46.351637254Z,
    total_entities: 363,
    vehicles: 363,
    trip_updates: 0,
    alerts: 0,
    shapes: 0,
    stops: 0,
    trip_modifications: 0,
    with_trip: 363,
    with_vehicle_descriptor: 363,
    with_position: 363,
    with_bearing: 315,
    with_speed: 45,
    with_odometer: 0,
    with_current_stop_sequence: 351,
    with_stop_id: 351,
    with_current_status: 363,
    with_timestamp: 363,
    with_congestion_level: 0,
    with_occupancy: 233,
    with_occupancy_percentage: 233,
    with_multi_carriage_details: 88,
}
```

## Configuration

### Environment Variables

Create a `.env` file in the project root with the following:

```env
MOBILITYDATA_REFRESH_TOKEN=your_refresh_token_here
```

To obtain a MobilityData refresh token:
1. Sign up at [mobilitydatabase.org](https://mobilitydatabase.org)
2. Get your refresh token from your Account Details page

## Dependencies

- `prost` - Protocol Buffers parsing
- `reqwest` - HTTP client for fetching remote feeds
- `serde` / `serde_json` - Serialization
- `tokio` - Async runtime
- `chrono` - Date/time handling
- `anyhow` - Error handling
- `clap` - Command-line argument parsing
- `csv` - CSV output
- `dotenvy` - Environment variable loading

## License

MIT License
