use anyhow::Result;

pub fn fetch_bytes(url: &str) -> Result<Vec<u8>> {
    let resp = reqwest::blocking::get(url)?;
    Ok(resp.bytes()?.to_vec())
}
