use crate::fetch::client::HttpClient;
use async_trait::async_trait;

pub struct ApiKey<C> {
    pub inner: C,
    pub key: String,
}

#[async_trait]
impl<C: HttpClient> HttpClient for ApiKey<C> {
    async fn execute(&self, mut req: reqwest::Request) -> reqwest::Result<reqwest::Response> {
        req.headers_mut().insert(
            "Authorization",
            format!("Bearer {}", self.key).parse().unwrap(),
        );

        self.inner.execute(req).await
    }
}
