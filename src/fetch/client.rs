use async_trait::async_trait;
use reqwest::{Request, Response};

#[async_trait]
pub trait HttpClient: Send + Sync {
    async fn execute(&self, req: Request) -> reqwest::Result<Response>;
}
