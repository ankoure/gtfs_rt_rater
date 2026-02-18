use async_trait::async_trait;
use reqwest::{Request, Response};

/// A trait abstracting HTTP request execution.
///
/// Implement this trait to provide custom HTTP behavior such as
/// authentication headers, retries, or request middleware.
#[async_trait]
pub trait HttpClient: Send + Sync {
    /// Sends an HTTP request and returns the response.
    async fn execute(&self, req: Request) -> reqwest::Result<Response>;
}
