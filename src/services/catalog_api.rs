//! Trait and types for interacting with a GTFS-RT feed catalog.

use anyhow::Result;

/// Metadata for a single GTFS-RT feed from the catalog.
#[derive(Debug, Clone)]
pub struct Feed {
    pub id: String,
    pub name: String,
    pub url: Option<String>,
    pub requires_auth: bool,
    pub status: Option<String>,
}

/// Abstraction over a feed catalog provider (e.g., MobilityData).
#[async_trait::async_trait]
pub trait CatalogApi {
    /// Returns all available GTFS-RT vehicle position feeds.
    async fn list_feeds(&self) -> Result<Vec<Feed>>;
}
