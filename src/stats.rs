use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::gtfs_rt::FeedMessage;

#[derive(Debug, Default, Serialize)]
pub struct FeedStats {
    pub timestamp: DateTime<Utc>,
    pub total_entities: usize,

    // entity types
    pub vehicles: usize,
    pub trip_updates: usize,
    pub alerts: usize,
    pub shapes: usize,
    pub stops: usize,
    pub trip_modifications: usize,

    // vehicle fields
    pub with_trip: usize,
    pub with_vehicle_descriptor: usize,
    pub with_position: usize,
    pub with_bearing: usize,
    pub with_speed: usize,
    pub with_odometer: usize,
    pub with_current_stop_sequence: usize,
    pub with_stop_id: usize,
    pub with_current_status: usize,
    pub with_timestamp: usize,
    pub with_congestion_level: usize,
    pub with_occupancy: usize,
    pub with_occupancy_percentage: usize,
    pub with_multi_carriage_details: usize,
}

impl FeedStats {
    pub fn from_feed(feed: &FeedMessage) -> Self {
        let mut s = FeedStats {
            timestamp: Utc::now(),
            total_entities: 0,
            vehicles: 0,
            trip_updates: 0,
            alerts: 0,
            shapes: 0,
            stops: 0,
            trip_modifications: 0,
            with_trip: 0,
            with_vehicle_descriptor: 0,
            with_position: 0,
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
        };

        s.total_entities = feed.entity.len();

        for e in &feed.entity {
            if let Some(v) = &e.vehicle {
                s.vehicles += 1;

                if v.trip.is_some() {
                    s.with_trip += 1;
                }

                if v.vehicle.is_some() {
                    s.with_vehicle_descriptor += 1;
                }

                if let Some(pos) = &v.position {
                    s.with_position += 1;

                    if pos.bearing.is_some() {
                        s.with_bearing += 1;
                    }

                    if pos.speed.is_some() {
                        s.with_speed += 1;
                    }

                    if pos.odometer.is_some() {
                        s.with_odometer += 1;
                    }
                }

                if v.current_stop_sequence.is_some() {
                    s.with_current_stop_sequence += 1;
                }

                if v.stop_id.is_some() {
                    s.with_stop_id += 1;
                }

                if v.current_status.is_some() {
                    s.with_current_status += 1;
                }

                if v.timestamp.is_some() {
                    s.with_timestamp += 1;
                }

                if v.congestion_level.is_some() {
                    s.with_congestion_level += 1;
                }

                if v.occupancy_status.is_some() {
                    s.with_occupancy += 1;
                }

                if v.occupancy_percentage.is_some() {
                    s.with_occupancy_percentage += 1;
                }

                if !v.multi_carriage_details.is_empty() {
                    s.with_multi_carriage_details += 1;
                }
            }

            if e.trip_update.is_some() {
                s.trip_updates += 1;
            }

            if e.alert.is_some() {
                s.alerts += 1;
            }

            if e.shape.is_some() {
                s.shapes += 1;
            }

            if e.stop.is_some() {
                s.stops += 1;
            }

            if e.trip_modifications.is_some() {
                s.trip_modifications += 1;
            }
        }

        s
    }

    pub fn pct(part: usize, total: usize) -> f64 {
        if total == 0 {
            0.0
        } else {
            (part as f64 / total as f64) * 100.0
        }
    }

    pub fn bearing_pct(&self) -> f64 {
        Self::pct(self.with_bearing, self.vehicles)
    }
}
