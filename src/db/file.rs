use anyhow::{Ok, Result};
use aws_config::{timeout::TimeoutConfig, BehaviorVersion, Region};
use aws_sdk_s3::{config::Credentials, Client};
use std::{env, time::Duration};
use tokio::sync::OnceCell;

static R2_CLIENT: OnceCell<Client> = OnceCell::const_new();
static R2_PUBLIC_URL: OnceCell<String> = OnceCell::const_new();
static R2_BUCKET: OnceCell<String> = OnceCell::const_new();

async fn get_client() -> &'static Client {
    R2_CLIENT
        .get_or_init(|| {
            Box::pin(async {
                let access_key = env::var("R2_ACCESS_KEY_ID").unwrap();
                let secret_key = env::var("R2_SECRET_ACCESS_KEY").unwrap();
                let endpoint = env::var("R2_ENDPOINT").unwrap();
                let region = env::var("R2_REGION").unwrap();

                let mut timeout = TimeoutConfig::builder();

                timeout
                    .set_connect_timeout(Some(Duration::from_secs(3)))
                    .set_operation_timeout(Some(Duration::from_secs(10)))
                    .set_read_timeout(Some(Duration::from_secs(3)));

                let config = aws_config::defaults(BehaviorVersion::latest())
                    .credentials_provider(Credentials::new(
                        access_key,
                        secret_key,
                        None,
                        None,
                        "rust-imgres",
                    ))
                    .region(Region::new(region))
                    .endpoint_url(endpoint)
                    .timeout_config(timeout.build())
                    .load()
                    .await;

                Client::new(&config)
            })
        })
        .await
}

async fn get_r2_buket() -> &'static String {
    R2_BUCKET
        .get_or_init(|| Box::pin(async { env::var("R2_BUCKET").unwrap() }))
        .await
}

async fn get_r2_public_url() -> &'static String {
    R2_PUBLIC_URL
        .get_or_init(|| Box::pin(async { env::var("R2_PUB").unwrap() }))
        .await
}

// resize_upload_temp
fn get_temp_key(filename: &str) -> String {
    format!("temp/resize_upload_{}", filename)
}

pub async fn upload_temp(blob: Vec<u8>, filename: &str) -> Result<String> {
    let client: &Client = get_client().await;

    let body = aws_sdk_s3::primitives::ByteStream::from(blob);
    let key = get_temp_key(filename);
    client
        .put_object()
        .bucket(get_r2_buket().await)
        .key(&key)
        .body(body)
        .send()
        .await?;

    let pub_url = get_r2_public_url().await;
    Ok(format!("https://{}/{}", pub_url, &key))
}
