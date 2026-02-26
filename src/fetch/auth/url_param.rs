use crate::fetch::client::HttpClient;
use async_trait::async_trait;

/// An [`HttpClient`] wrapper that appends an API key as a URL query parameter.
///
/// `param_name` is the query parameter name (e.g. `"api_key"`) and `key` is
/// its value. Both come from the `api_key_parameter_name` field in the
/// MobilityDatabase catalog for feeds with `authentication_type = 1`.
pub struct UrlParam<C> {
    pub inner: C,
    pub param_name: String,
    pub key: String,
}

#[async_trait]
impl<C: HttpClient> HttpClient for UrlParam<C> {
    async fn execute(&self, mut req: reqwest::Request) -> reqwest::Result<reqwest::Response> {
        req.url_mut()
            .query_pairs_mut()
            .append_pair(&self.param_name, &self.key);
        self.inner.execute(req).await
    }
}
