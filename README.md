Based on the project structure and code, here's a README for the project:

# GTFS Realtime Rater

A Rust CLI tool for parsing and analyzing GTFS Realtime feeds. It calculates statistics about the completeness and quality of transit feed data.

## Features

- Parse GTFS Realtime protobuf feeds from files or URLs
- Calculate statistics on feed entities (vehicles, trip updates, alerts, etc.)
- Analyze field coverage for vehicle positions (bearing, speed, occupancy, etc.)
- Output results in pretty-print or JSON format
- Output results to CSV

## Requirements

- Rust (edition 2024)

## Installation
```bash
cargo build --release
```
## Usage

Analyze a local GTFS-RT feed file:

```bash
cargo run -- path/to/feed.pb
```



Analyze a GTFS-RT feed from a URL:

```shell script
cargo run -- https://cdn.mbta.com/realtime/VehiclePositions.pb
```


## Output

The tool outputs statistics including:

- **Entity counts**: vehicles, trip updates, alerts, shapes, stops, trip modifications
- **Vehicle field coverage**: position, bearing, speed, odometer, occupancy, timestamps, etc.

Example output:

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

## Dependencies

- `prost` - Protocol Buffers parsing
- `reqwest` - HTTP client for fetching remote feeds
- `serde` / `serde_json` - Serialization
- `tokio` - Async runtime
- `chrono` - Date/time handling
- `anyhow` - Error handling

## License

MIT License
