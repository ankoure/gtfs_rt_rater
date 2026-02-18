//! Feed data aggregation and quality grading.
//!
//! This module collects per-sample CSV data, computes weighted averages
//! for each optional GTFS-RT field, assigns letter grades, and uploads
//! the results as JSON to S3.

pub mod aggregate;
pub mod analyzer;
pub mod grade;
pub mod types;
pub mod utility;
pub mod writetos3;
