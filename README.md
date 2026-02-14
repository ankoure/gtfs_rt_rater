Based on the project structure and code, here's a README for the project:

# GTFS Realtime Rater

A Rust CLI tool for parsing and analyzing GTFS Realtime feeds. It calculates statistics about the completeness and quality of transit feed data.

## Features

- Parse GTFS Realtime protobuf feeds from files or URLs
- Calculate statistics on feed entities (vehicles, trip updates, alerts, etc.)
- Analyze field coverage for vehicle positions (bearing, speed, occupancy, etc.)
- Output results in pretty-print or JSON format

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
feed_stats {
    total_entities: 321,
    vehicles: 321,
    trip_updates: 0,
    alerts: 0,
    shapes: 0,
    stops: 0,
    trip_modifications: 0,
    with_trip: 321,
    with_vehicle_descriptor: 321,
    with_position: 321,
    with_bearing: 278,
    with_speed: 42,
    with_odometer: 0,
    with_current_stop_sequence: 308,
    with_stop_id: 308,
    with_current_status: 321,
    with_timestamp: 321,
    with_congestion_level: 0,
    with_occupancy: 194,
    with_occupancy_percentage: 194,
    with_multi_carriage_details: 89,
}
```

## Dependencies

- `prost` - Protocol Buffers parsing
- `reqwest` - HTTP client for fetching remote feeds
- `serde` / `serde_json` - Serialization
- `sqlx` - PostgreSQL database access
- `tokio` - Async runtime
- `chrono` - Date/time handling
- `anyhow` - Error handling

## License

MIT License
