use super::client::HttpClient;
use async_trait::async_trait;
use std::time::Duration;

pub struct BasicClient(reqwest::Client);

impl BasicClient {
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
