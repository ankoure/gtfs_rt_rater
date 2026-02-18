use super::client::HttpClient;
use async_trait::async_trait;
use std::time::Duration;

/// A simple [`HttpClient`](super::HttpClient) implementation with a 30-second request timeout
/// and a 10-second connection timeout.
pub struct BasicClient(reqwest::Client);

impl BasicClient {
    /// Creates a new `BasicClient` with default timeout settings.
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to build HTTP client");
        Self(client)
    }
}

#[async_trait]
impl HttpClient for BasicClient {
    async fn execute(&self, req: reqwest::Request) -> reqwest::Result<reqwest::Response> {
        self.0.execute(req).await
    }
}
