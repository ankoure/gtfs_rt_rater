use anyhow::Result;
use prost::Message;

use crate::gtfs_rt::FeedMessage;

pub fn parse_feed(bytes: &[u8]) -> Result<FeedMessage> {
    Ok(FeedMessage::decode(bytes)?)
}
