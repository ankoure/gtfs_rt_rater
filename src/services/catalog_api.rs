use anyhow::Result;

#[derive(Debug, Clone)]
pub struct Feed {
    pub id: String,
    pub name: String,
    pub url: Option<String>,
    pub requires_auth: bool,
    pub status: Option<String>,
}

#[async_trait::async_trait]
pub trait CatalogApi {
    async fn list_feeds(&self) -> Result<Vec<Feed>>;
}
