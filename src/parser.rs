//! Protobuf parser for GTFS Realtime feeds.

use anyhow::Result;
use prost::Message;

use crate::gtfs_rt::FeedMessage;

/// Decodes a protobuf-encoded GTFS-RT [`FeedMessage`] from raw bytes.
///
/// # Errors
///
/// Returns an error if the bytes are not valid protobuf for a `FeedMessage`.
pub fn parse_feed(bytes: &[u8]) -> Result<FeedMessage> {
    Ok(FeedMessage::decode(bytes)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_bytes_returns_default_feed() {
        // An empty byte array decodes to a FeedMessage with default values
        // This is valid protobuf behavior
        let result = parse_feed(&[]);
        assert!(result.is_ok());
        let feed = result.unwrap();
        assert_eq!(feed.header.gtfs_realtime_version, "");
        assert!(feed.entity.is_empty());
    }

    #[test]
    fn test_parse_invalid_bytes() {
        // Random invalid bytes should fail
        let invalid_bytes = vec![0xFF, 0xFE, 0x00, 0x01];
        let result = parse_feed(&invalid_bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_valid_minimal_feed() {
        // Create a minimal valid FeedMessage and encode it
        use crate::gtfs_rt::{FeedHeader, FeedMessage};

        let feed = FeedMessage {
            header: FeedHeader {
                gtfs_realtime_version: "2.0".to_string(),
                timestamp: Some(1234567890),
                incrementality: None,
                feed_version: None,
            },
            entity: vec![],
        };
        let encoded = feed.encode_to_vec();
        let result = parse_feed(&encoded);

        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed.header.gtfs_realtime_version, "2.0");
        assert_eq!(parsed.header.timestamp, Some(1234567890));
    }
}
