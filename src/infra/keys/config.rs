use anyhow::Result;
use std::collections::HashMap;

/// Maps feed IDs to SSM parameter paths (or any other vault reference).
///
/// Stored as a plain JSON object on disk:
/// ```json
/// {
///   "mdb-123": "/gtfs/feeds/mdb-123/api_key",
///   "mdb-456": "/gtfs/feeds/mdb-456/api_key"
/// }
/// ```
pub struct FeedKeyConfig {
    entries: HashMap<String, String>,
}

impl FeedKeyConfig {
    /// Loads the config from a JSON file at `path`.
    pub fn load(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let entries: HashMap<String, String> = serde_json::from_str(&content)?;
        Ok(Self { entries })
    }

    /// Returns the vault reference for `feed_id`, if one is configured.
    pub fn get_ref(&self, feed_id: &str) -> Option<&str> {
        self.entries.get(feed_id).map(String::as_str)
    }

    /// Iterates over all `(feed_id, reference)` pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.entries.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }
}
