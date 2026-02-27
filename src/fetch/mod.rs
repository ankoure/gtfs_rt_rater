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
use tracing::{debug, warn};

/// Fetches raw bytes from the given URL using the provided HTTP client.
#[tracing::instrument(skip(client), fields(url, bytes_received))]
pub async fn fetch_bytes<C: HttpClient>(client: &C, url: &str) -> Result<Vec<u8>> {
    debug!(url, "Sending HTTP GET");

    let req = reqwest::Request::new(reqwest::Method::GET, url.parse()?);
    let resp = client.execute(req).await?;

    let status = resp.status();
    if !status.is_success() {
        warn!(url, status = %status, "HTTP response non-success");
    }

    let bytes = resp.bytes().await?.to_vec();
    tracing::Span::current().record("bytes_received", bytes.len());
    debug!(url, bytes = bytes.len(), "HTTP GET complete");

    Ok(bytes)
}
