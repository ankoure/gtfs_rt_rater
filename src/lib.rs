//! # GTFS-RT Rater
//!
//! A library for fetching, parsing, and analyzing [GTFS Realtime](https://gtfs.org/realtime/) feeds.
//!
//! This crate provides tools to evaluate the data quality of GTFS-RT vehicle position feeds
//! by measuring field completeness (bearing, speed, occupancy, etc.) and computing
//! aggregate scores and letter grades.
//!
//! ## Modules
//!
//! - [`fetch`] - HTTP client abstractions for downloading feed data
//! - [`parser`] - Protobuf deserialization of GTFS-RT `FeedMessage`s
//! - [`stats`] - Per-sample statistics extracted from a single feed snapshot
//! - [`output`] - CSV and JSON serialization of feed statistics
//! - [`analyzers`] - Aggregation, grading, and S3 upload of collected data

pub mod analyzers;
pub mod fetch;
pub mod output;
pub mod parser;
pub mod stats;

/// Auto-generated protobuf types from the GTFS Realtime specification.
pub mod gtfs_rt {
    include!(concat!(env!("OUT_DIR"), "/transit_realtime.rs"));
}
