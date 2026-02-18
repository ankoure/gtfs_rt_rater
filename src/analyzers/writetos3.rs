use serde::Serialize;

/// Serializes a value to JSON and uploads it to an S3 bucket with `application/json` content type.
pub async fn write_json_to_s3(
    client: &aws_sdk_s3::Client,
    bucket: &str,
    key: &str,
    value: &impl Serialize,
) -> anyhow::Result<()> {
    let body = serde_json::to_vec(value)?;

    client
        .put_object()
        .bucket(bucket)
        .key(key)
        .body(body.into())
        .content_type("application/json")
        .send()
        .await?;

    Ok(())
}
