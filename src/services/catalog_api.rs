//! Trait and types for interacting with a GTFS-RT feed catalog.

use anyhow::Result;

/// Describes how a feed requires authentication.
///
/// Maps directly to the MobilityDatabase `authentication_type` field:
/// - `0` → [`FeedAuth::None`]
/// - `1` → [`FeedAuth::UrlParam`] – API key appended as a query parameter
/// - `2` → [`FeedAuth::Header`] – API key sent as an HTTP header
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeedAuth {
    /// No authentication required.
    None,
    /// API key must be appended as a URL query parameter with the given name.
    UrlParam { param_name: String },
    /// API key must be sent as an HTTP header with the given name.
    Header { header_name: String },
}

impl FeedAuth {
    /// Returns `true` if any authentication credentials are needed.
    pub fn requires_auth(&self) -> bool {
        !matches!(self, FeedAuth::None)
    }
}

/// Metadata for a single GTFS-RT feed from the catalog.
#[derive(Debug, Clone)]
pub struct Feed {
    pub id: String,
    pub name: String,
    pub url: Option<String>,
    pub auth: FeedAuth,
    pub status: Option<String>,
}

/// Abstraction over a feed catalog provider (e.g., MobilityData).
#[async_trait::async_trait]
pub trait CatalogApi {
    /// Returns all available GTFS-RT vehicle position feeds.
    async fn list_feeds(&self) -> Result<Vec<Feed>>;
}
