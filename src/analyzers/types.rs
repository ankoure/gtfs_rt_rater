//! Data types used by the aggregation pipeline.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single row deserialized from a per-feed CSV file.
#[derive(Debug, Deserialize)]
pub struct FeedStats {
    pub(crate) timestamp: DateTime<Utc>,
    pub(crate) vehicles: usize,
    pub(crate) error_type: Option<String>,

    pub(crate) with_trip_id: usize,
    pub(crate) with_route_id: usize,
    pub(crate) with_direction_id: usize,

    pub(crate) with_vehicle_id: usize,
    pub(crate) with_vehicle_label: usize,
    pub(crate) with_license_plate: usize,
    pub(crate) with_wheelchair_accessible: usize,

    pub(crate) with_bearing: usize,
    pub(crate) with_speed: usize,
    pub(crate) with_odometer: usize,
    pub(crate) with_current_stop_sequence: usize,
    pub(crate) with_stop_id: usize,
    pub(crate) with_current_status: usize,
    pub(crate) with_timestamp: usize,
    pub(crate) with_congestion_level: usize,
    pub(crate) with_occupancy: usize,
    pub(crate) with_occupancy_percentage: usize,
    pub(crate) with_multi_carriage_details: usize,
}
/// Aggregated statistics for a single optional vehicle field.
#[derive(Serialize)]
pub struct FieldAggregate {
    pub(crate) avg_support: f64,
    pub(crate) stddev: f64,
    pub(crate) grade: String,
}

/// High-level entity statistics: average vehicle count, uptime, and service time.
#[derive(Serialize)]
pub struct EntityStats {
    pub(crate) avg_vehicles: f64,
    pub(crate) uptime_percent: f64,
    pub(crate) service_time_percent: f64,
}

/// Overall weighted score and letter grade for a feed.
#[derive(Serialize)]
pub struct OverallAggregate {
    pub(crate) score: f64,
    pub(crate) grade: String,
}

/// Complete aggregation result for a single feed, uploaded as JSON to S3.
#[derive(Serialize)]
pub struct FeedAggregate {
    pub(crate) schema_version: u8,
    pub(crate) algorithm_version: u8,
    pub(crate) feed_id: String,
    pub(crate) last_updated: DateTime<Utc>,
    pub(crate) window_minutes: i64,
    pub(crate) entity_stats: EntityStats,
    pub(crate) fields: HashMap<String, FieldAggregate>,
    pub(crate) overall: OverallAggregate,
}

/// Summary entry for the feed index listing.
#[derive(Serialize)]
pub struct FeedIndexEntry {
    pub(crate) feed_id: String,
    pub(crate) overall_grade: String,
    pub(crate) overall_score: f64,
    pub(crate) uptime_percent: f64,
}

/// Top-level index of all aggregated feeds, served as `aggregates/feeds.json`.
#[derive(Serialize)]
pub struct FeedIndex {
    pub(crate) generated_at: DateTime<Utc>,
    pub(crate) feeds: Vec<FeedIndexEntry>,
}
