use crate::analyzers::grade::grade;
use crate::analyzers::types::{
    EntityStats, FeedAggregate, FeedStats, FieldAggregate, OverallAggregate,
};
use crate::analyzers::utility::{mean, stddev};
use chrono::Utc;
use std::collections::HashMap;

/// Weights used in the weighted average for each field and uptime.
/// Higher weight means the field contributes more to the overall score.
static WEIGHTS: &[(&str, f64)] = &[
    ("bearing", 1.0),
    ("speed", 1.0),
    ("occupancy", 2.0),
    ("stop_sequence", 2.0),
    ("multi_carriage", 1.0),
    ("odometer", 1.0),
    ("stop_id", 1.0),
    ("current_status", 1.0),
    ("timestamp", 1.0),
    ("congestion_level", 1.0),
    ("occupancy_percentage", 1.0),
    ("uptime", 3.0),
];

/// Aggregates a series of [`FeedStats`] rows into a single [`FeedAggregate`].
///
/// Computes per-field support averages, standard deviations, letter grades,
/// and an overall weighted score incorporating uptime.
pub fn aggregate_feed(feed_id: &str, rows: Vec<FeedStats>) -> anyhow::Result<FeedAggregate> {
    let now = Utc::now();

    let window_minutes = if rows.len() < 2 {
        0
    } else {
        let first = rows.first().unwrap().timestamp;
        let last = rows.last().unwrap().timestamp;
        (last - first).num_minutes()
    };

    let mut uptime_minutes = 0i64;
    let mut vehicle_counts = Vec::new();

    let mut field_series: HashMap<&str, Vec<f64>> = HashMap::new();

    for row in &rows {
        if row.vehicles == 0 {
            continue;
        }

        uptime_minutes += 1;
        vehicle_counts.push(row.vehicles as f64);

        macro_rules! push_field {
            ($name:expr, $value:expr) => {
                field_series
                    .entry($name)
                    .or_default()
                    .push($value as f64 / row.vehicles as f64);
            };
        }

        push_field!("bearing", row.with_bearing);
        push_field!("speed", row.with_speed);
        push_field!("occupancy", row.with_occupancy);
        push_field!("stop_sequence", row.with_current_stop_sequence);
        push_field!("multi_carriage", row.with_multi_carriage_details);
        push_field!("odometer", row.with_odometer);
        push_field!("stop_id", row.with_stop_id);
        push_field!("current_status", row.with_current_status);
        push_field!("timestamp", row.with_timestamp);
        push_field!("congestion_level", row.with_congestion_level);
        push_field!("occupancy_percentage", row.with_occupancy_percentage);
    }

    let avg_vehicles = mean(&vehicle_counts);
    let uptime_percent = if window_minutes == 0 {
        0.0
    } else {
        uptime_minutes as f64 / window_minutes as f64
    };

    let weights: HashMap<&str, f64> = WEIGHTS.iter().copied().collect();

    let mut fields = HashMap::new();
    let mut weighted_total = 0.0;
    let mut weight_sum = 0.0;

    for (name, series) in field_series {
        if series.is_empty() {
            continue;
        }

        let avg = mean(&series);
        let sd = stddev(&series, avg);

        let weight = *weights.get(name).unwrap_or(&1.0);

        weighted_total += avg * weight;
        weight_sum += weight;

        fields.insert(
            name.to_string(),
            FieldAggregate {
                avg_support: avg,
                stddev: sd,
                grade: grade(avg),
            },
        );
    }

    // Factor uptime into overall score
    let uptime_weight = *weights.get("uptime").unwrap_or(&3.0);
    weighted_total += uptime_percent * uptime_weight;
    weight_sum += uptime_weight;

    let overall_score = if weight_sum == 0.0 {
        0.0
    } else {
        weighted_total / weight_sum
    };

    Ok(FeedAggregate {
        schema_version: 1,
        algorithm_version: 1,
        feed_id: feed_id.to_string(),
        last_updated: now,
        window_minutes,
        entity_stats: EntityStats {
            avg_vehicles,
            uptime_percent,
        },
        fields,
        overall: OverallAggregate {
            score: overall_score,
            grade: grade(overall_score),
        },
    })
}
