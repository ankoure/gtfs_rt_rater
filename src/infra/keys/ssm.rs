use anyhow::{Context, Result};

use super::KeyStore;

/// Resolves secrets from AWS SSM Parameter Store.
///
/// Parameters are fetched with decryption enabled, so `SecureString` values
/// work out of the box as long as the process has `ssm:GetParameter` and the
/// corresponding KMS permissions.
pub struct SsmKeyStore {
    client: aws_sdk_ssm::Client,
}

impl SsmKeyStore {
    /// Creates a store using the ambient AWS configuration (env vars, instance
    /// profile, etc.) already loaded by `aws_config::load_from_env`.
    pub fn new(config: &aws_config::SdkConfig) -> Self {
        Self {
            client: aws_sdk_ssm::Client::new(config),
        }
    }
}

#[async_trait::async_trait]
impl KeyStore for SsmKeyStore {
    /// Fetches the parameter at `reference` (an SSM path such as
    /// `/gtfs/feeds/mdb-123/api_key`) and returns its plaintext value.
    async fn get(&self, reference: &str) -> Result<String> {
        let resp = self
            .client
            .get_parameter()
            .name(reference)
            .with_decryption(true)
            .send()
            .await
            .with_context(|| format!("SSM GetParameter failed for '{reference}'"))?;

        resp.parameter
            .and_then(|p| p.value)
            .ok_or_else(|| anyhow::anyhow!("SSM parameter '{reference}' exists but has no value"))
    }
}
