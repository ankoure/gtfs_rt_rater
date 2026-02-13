use super::client::HttpClient;
use async_trait::async_trait;

pub struct BasicClient(reqwest::Client);

impl BasicClient {
    pub fn new() -> Self {
        Self(reqwest::Client::new())
    }
}

#[async_trait]
impl HttpClient for BasicClient {
    async fn execute(&self, req: reqwest::Request) -> reqwest::Result<reqwest::Response> {
        self.0.execute(req).await
    }
}
