//! HTTP client abstractions for fetching GTFS-RT feed data.
//!
//! Provides the [`HttpClient`] trait for pluggable HTTP backends and a
//! [`BasicClient`] implementation with sensible timeouts.

pub mod auth;
mod basic;
mod client;

pub use basic::BasicClient;
pub use client::HttpClient;

use anyhow::Result;

/// Fetches raw bytes from the given URL using the provided HTTP client.
pub async fn fetch_bytes<C: HttpClient>(client: &C, url: &str) -> Result<Vec<u8>> {
    let req = reqwest::Request::new(reqwest::Method::GET, url.parse()?);

    let resp = client.execute(req).await?;
    Ok(resp.bytes().await?.to_vec())
}
