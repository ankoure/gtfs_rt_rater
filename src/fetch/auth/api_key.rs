use crate::fetch::client::HttpClient;
use async_trait::async_trait;
use reqwest::header::HeaderName;

/// An [`HttpClient`] wrapper that injects an API key as an HTTP header.
///
/// `header_name` is the header field to set (e.g. `"Authorization"` or a
/// provider-specific name returned by the MobilityDatabase catalog).
/// `key` is the raw value written into that header.
pub struct ApiKey<C> {
    pub inner: C,
    pub header_name: String,
    pub key: String,
}

impl<C> ApiKey<C> {
    /// Convenience constructor that uses `Authorization: Bearer <key>`, the
    /// most common pattern for OAuth-style tokens.
    pub fn bearer(inner: C, key: String) -> Self {
        Self {
            inner,
            header_name: "Authorization".to_string(),
            key: format!("Bearer {key}"),
        }
    }
}

#[async_trait]
impl<C: HttpClient> HttpClient for ApiKey<C> {
    async fn execute(&self, mut req: reqwest::Request) -> reqwest::Result<reqwest::Response> {
        let header_name = HeaderName::from_bytes(self.header_name.as_bytes())
            .expect("ApiKey: invalid header name");
        req.headers_mut()
            .insert(header_name, self.key.parse().unwrap());
        self.inner.execute(req).await
    }
}
