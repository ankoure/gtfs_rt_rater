//! Feed API-key management.
//!
//! [`FeedKeyConfig`] maps feed IDs to references (SSM parameter paths).
//! [`KeyStore`] is the async trait for resolving a reference into its plaintext value.
//! [`SsmKeyStore`] implements [`KeyStore`] using AWS SSM Parameter Store.

mod config;
mod ssm;

pub use config::FeedKeyConfig;
pub use ssm::SsmKeyStore;

use anyhow::Result;

/// Resolves a vault reference (e.g. an SSM parameter path) into a plaintext secret.
#[async_trait::async_trait]
pub trait KeyStore: Send + Sync {
    async fn get(&self, reference: &str) -> Result<String>;
}
