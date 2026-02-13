mod client;
mod basic;
pub mod auth;

pub use client::HttpClient;
pub use basic::BasicClient;

use anyhow::Result;

pub async fn fetch_bytes<C: HttpClient>(
    client: &C,
    url: &str,
) -> Result<Vec<u8>> {
    let req = reqwest::Request::new(
        reqwest::Method::GET,
        url.parse()?,
    );

    let resp = client.execute(req).await?;
    Ok(resp.bytes().await?.to_vec())
}
