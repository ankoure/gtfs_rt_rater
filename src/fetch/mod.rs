pub mod auth;
mod basic;
mod client;

pub use basic::BasicClient;
pub use client::HttpClient;

use anyhow::Result;

pub async fn fetch_bytes<C: HttpClient>(client: &C, url: &str) -> Result<Vec<u8>> {
    let req = reqwest::Request::new(reqwest::Method::GET, url.parse()?);

    let resp = client.execute(req).await?;
    Ok(resp.bytes().await?.to_vec())
}
