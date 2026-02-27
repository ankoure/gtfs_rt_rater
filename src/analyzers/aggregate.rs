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
    ("route_id", 3.0),
    ("direction_id", 3.0),
    ("stop_id", 3.0),
    ("stop_sequence", 3.0),
    ("trip_id", 2.0),
    ("vehicle_id", 0.0),
    ("vehicle_label", 0.0),
    ("license_plate", 0.0),
    ("wheelchair_accessible", 0.0),
    ("bearing", 0.0),
    ("speed", 0.0),
    ("occupancy", 1.0),
    ("multi_carriage", 0.0),
    ("odometer", 0.0),
    ("current_status", 1.0),
    ("timestamp", 1.0),
    ("congestion_level", 0.0),
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

    // Uptime: fraction of polling attempts where the API responded without error.
    let successful_polls = rows
        .iter()
        .filter(|r| r.error_type.as_deref().map_or(true, |s| s.is_empty()))
        .count();
    let uptime_percent = if rows.is_empty() {
        0.0
    } else {
        successful_polls as f64 / rows.len() as f64
    };

    // Service time: fraction of polling attempts where at least one vehicle was present.
    let service_polls = rows.iter().filter(|r| r.vehicles > 0).count();
    let service_time_percent = if rows.is_empty() {
        0.0
    } else {
        service_polls as f64 / rows.len() as f64
    };

    let mut vehicle_counts = Vec::new();

    let mut field_series: HashMap<&str, Vec<f64>> = HashMap::new();

    for row in &rows {
        if row.vehicles == 0 {
            continue;
        }

        vehicle_counts.push(row.vehicles as f64);

        macro_rules! push_field {
            ($name:expr, $value:expr) => {
                field_series
                    .entry($name)
                    .or_default()
                    .push($value as f64 / row.vehicles as f64);
            };
        }

        push_field!("trip_id", row.with_trip_id);
        push_field!("route_id", row.with_route_id);
        push_field!("direction_id", row.with_direction_id);
        push_field!("vehicle_id", row.with_vehicle_id);
        push_field!("vehicle_label", row.with_vehicle_label);
        push_field!("license_plate", row.with_license_plate);
        push_field!("wheelchair_accessible", row.with_wheelchair_accessible);
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
        algorithm_version: 2,
        feed_id: feed_id.to_string(),
        last_updated: now,
        window_minutes,
        entity_stats: EntityStats {
            avg_vehicles,
            uptime_percent,
            service_time_percent,
        },
        fields,
        overall: OverallAggregate {
            score: overall_score,
            grade: grade(overall_score),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    /// Builds a minimal `FeedStats` row with all counts zeroed out.
    /// Override only the fields relevant to each test.
    fn make_row(vehicles: usize, error: bool) -> FeedStats {
        FeedStats {
            timestamp: Utc::now(),
            vehicles,
            error_type: if error {
                Some("fetch_error".to_string())
            } else {
                None
            },
            with_trip_id: 0,
            with_route_id: 0,
            with_direction_id: 0,
            with_vehicle_id: 0,
            with_vehicle_label: 0,
            with_license_plate: 0,
            with_wheelchair_accessible: 0,
            with_bearing: 0,
            with_speed: 0,
            with_odometer: 0,
            with_current_stop_sequence: 0,
            with_stop_id: 0,
            with_current_status: 0,
            with_timestamp: 0,
            with_congestion_level: 0,
            with_occupancy: 0,
            with_occupancy_percentage: 0,
            with_multi_carriage_details: 0,
        }
    }

    #[test]
    fn test_empty_rows() {
        let result = aggregate_feed("test-feed", vec![]).unwrap();
        assert_eq!(result.entity_stats.uptime_percent, 0.0);
        assert_eq!(result.entity_stats.service_time_percent, 0.0);
        assert_eq!(result.overall.score, 0.0);
        assert!(result.fields.is_empty());
    }

    #[test]
    fn test_all_error_rows() {
        let rows = vec![make_row(0, true), make_row(0, true)];
        let result = aggregate_feed("test-feed", rows).unwrap();
        assert_eq!(result.entity_stats.uptime_percent, 0.0);
    }

    #[test]
    fn test_uptime_fraction() {
        // 3 successful, 1 error → uptime = 0.75
        let rows = vec![
            make_row(10, false),
            make_row(10, false),
            make_row(10, false),
            make_row(0, true),
        ];
        let result = aggregate_feed("test-feed", rows).unwrap();
        assert!((result.entity_stats.uptime_percent - 0.75).abs() < 1e-10);
    }

    #[test]
    fn test_service_time_fraction() {
        // 2 rows with vehicles, 2 without → service_time = 0.5
        let rows = vec![
            make_row(5, false),
            make_row(5, false),
            make_row(0, false),
            make_row(0, false),
        ];
        let result = aggregate_feed("test-feed", rows).unwrap();
        assert!((result.entity_stats.service_time_percent - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_field_avg_support() {
        // 1 vehicle, route_id present → route_id avg_support should be 1.0
        let mut row = make_row(1, false);
        row.with_route_id = 1;
        let result = aggregate_feed("test-feed", vec![row]).unwrap();
        let route = result.fields.get("route_id").unwrap();
        assert!((route.avg_support - 1.0).abs() < 1e-10);
        assert_eq!(route.grade, "A+");
    }

    #[test]
    fn test_no_vehicles_rows_skipped_for_fields() {
        // Rows with vehicles=0 should not contribute to field averages
        let rows = vec![make_row(0, false), make_row(0, false)];
        let result = aggregate_feed("test-feed", rows).unwrap();
        assert!(result.fields.is_empty());
    }

    #[test]
    fn test_partial_field_support() {
        // 4 vehicles, only 2 have route_id → avg_support = 0.5
        let mut row = make_row(4, false);
        row.with_route_id = 2;
        let result = aggregate_feed("test-feed", vec![row]).unwrap();
        let route = result.fields.get("route_id").unwrap();
        assert!((route.avg_support - 0.5).abs() < 1e-10);
        assert_eq!(route.grade, "D"); // 0.5 >= 0.40 → D
    }

    #[test]
    fn test_avg_vehicles() {
        // Two rows with 4 and 8 vehicles → avg = 6.0
        let rows = vec![make_row(4, false), make_row(8, false)];
        let result = aggregate_feed("test-feed", rows).unwrap();
        assert!((result.entity_stats.avg_vehicles - 6.0).abs() < 1e-10);
    }

    #[test]
    fn test_window_minutes() {
        use chrono::Duration;
        let t0 = Utc::now();
        let mut row1 = make_row(5, false);
        row1.timestamp = t0;
        let mut row2 = make_row(5, false);
        row2.timestamp = t0 + Duration::minutes(45);
        let result = aggregate_feed("test-feed", vec![row1, row2]).unwrap();
        assert_eq!(result.window_minutes, 45);
    }

    #[test]
    fn test_single_row_window_is_zero() {
        let result = aggregate_feed("test-feed", vec![make_row(5, false)]).unwrap();
        assert_eq!(result.window_minutes, 0);
    }

    #[test]
    fn test_overall_score_full_uptime_no_vehicle_data() {
        // No vehicle rows → only uptime contributes to weighted score.
        // uptime=1.0, uptime_weight=3.0 → score = 3.0/3.0 = 1.0
        let rows = vec![make_row(0, false), make_row(0, false)];
        let result = aggregate_feed("test-feed", rows).unwrap();
        assert!((result.overall.score - 1.0).abs() < 1e-10);
        assert_eq!(result.overall.grade, "A+");
    }

    #[test]
    fn test_feed_id_preserved() {
        let result = aggregate_feed("my-agency-feed", vec![]).unwrap();
        assert_eq!(result.feed_id, "my-agency-feed");
    }

    #[test]
    fn test_field_stddev_nonzero() {
        // Row 1: all 4 vehicles have route_id → support = 1.0
        // Row 2: 0 of 4 have route_id → support = 0.0
        // mean = 0.5, population stddev = 0.5
        let mut row1 = make_row(4, false);
        row1.with_route_id = 4;
        let row2 = make_row(4, false);
        let result = aggregate_feed("test-feed", vec![row1, row2]).unwrap();
        let route = result.fields.get("route_id").unwrap();
        assert!((route.stddev - 0.5).abs() < 1e-10);
    }
}
