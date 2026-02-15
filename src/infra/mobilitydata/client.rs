use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::services::catalog_api::{CatalogApi, Feed};

#[derive(Serialize)]
struct TokenRequest {
    refresh_token: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

pub struct MobilityDataClient {
    base_url: String,
    access_token: String,
}

impl MobilityDataClient {
    pub async fn new(refresh_token: String) -> Result<Self> {
        // Exchange refresh token for access token
        let access_token = Self::exchange_token(&refresh_token).await?;

        Ok(Self {
            base_url: "https://api.mobilitydatabase.org".to_string(),
            access_token,
        })
    }

    async fn exchange_token(refresh_token: &str) -> Result<String> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .build()?;

        let token_request = TokenRequest {
            refresh_token: refresh_token.to_string(),
        };

        let response = client
            .post("https://api.mobilitydatabase.org/v1/tokens")
            .header("Content-Type", "application/json")
            .json(&token_request)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send token request: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Token exchange failed with status {}: {}", status, body));
        }

        let token_response: TokenResponse = response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse token response: {}", e))?;

        Ok(token_response.access_token)
    }
}

#[async_trait]
impl CatalogApi for MobilityDataClient {
    async fn list_feeds(&self) -> Result<Vec<Feed>> {
        let url = format!(
            "{}/v1/gtfs_rt_feeds?limit=999&offset=0&entity_types=vp",
            self.base_url
        );

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .build()?;

        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send request: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("API returned status {}: {}", status, body));
        }

        // Parse as generic JSON to extract only the fields we need
        let json: Vec<serde_json::Value> = response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?;

        let feeds = json
            .into_iter()
            .filter_map(|item| {
                let id = item["id"].as_str()?.to_string();
                let name = item["provider"].as_str().unwrap_or("").to_string();
                let url = item["source_info"]["producer_url"].as_str().map(|s| s.to_string());
                let auth_type = item["source_info"]["authentication_type"].as_i64().unwrap_or(0);
                let requires_auth = auth_type != 0;
                let status = item["status"].as_str().map(|s| s.to_string());

                Some(Feed { id, name, url, requires_auth, status })
            })
            .collect();

        Ok(feeds)
    }
}