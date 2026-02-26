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

/// Allows a `Box<dyn HttpClient>` to be passed wherever `HttpClient` is
/// expected, enabling runtime-selected auth strategies.
#[async_trait]
impl HttpClient for Box<dyn HttpClient> {
    async fn execute(&self, req: Request) -> reqwest::Result<Response> {
        (**self).execute(req).await
    }
}
