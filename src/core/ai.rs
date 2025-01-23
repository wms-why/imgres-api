use std::sync::OnceLock;
use std::time::Duration;
use std::{collections::HashMap, env};

use anyhow::{Ok, Result};

use bytes::Bytes;
use reqwest::header::HeaderMap;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use tokio::time;

static CLIENT: OnceLock<Client> = OnceLock::new();
static MODEL_VERSION: OnceLock<String> = OnceLock::new();

fn get_client() -> Client {
    reqwest::ClientBuilder::new().build().unwrap()
}

fn get_headers() -> HeaderMap {
    let token = env::var("REPLICATE_API_TOKEN").unwrap();
    let mut headers = HashMap::with_capacity(3);
    headers.insert("Authorization".to_string(), format!("Bearer {}", token));
    headers.insert("content-type".to_string(), "application/json".to_string());
    headers.insert("Prefer".to_string(), "wait".to_string());

    (&headers).try_into().unwrap()
}

fn get_model_version() -> String {
    env::var("REPLICATE_MODEL_VERSION").unwrap()
}

///
/// nightmareai/real-esrgan
///
/// curl -s -X POST \
/// -H "Authorization: Bearer $REPLICATE_API_TOKEN" \
/// -H "Content-Type: application/json" \
/// -H "Prefer: wait" \
/// -d $'{
///   "version": "f121d640bd286e1fdc67f9799164c1d5be36ff74576ee11c803ae5b665dd46aa",
///   "input": {
///     "scale": 2.85,
///     "face_enhance": false
///   }
/// }' \
/// https://api.replicate.com/v1/predictions
pub async fn resize(img_src: &str, scale_factor: f32) -> Result<Bytes> {
    let client = CLIENT.get_or_init(get_client);
    let body = ReqBody::new(img_src, scale_factor);
    let r = client
        .post("https://api.replicate.com/v1/predictions")
        .headers(get_headers())
        .body(serde_json::to_string(&body)?);

    let r = r.send().await?;
    let status = r.status().as_u16();
    let text = r.text().await?;

    if (200..300).contains(&status) {
        let result = serde_json::from_str::<RespBody>(&text);

        if result.is_err() {
            return Err(anyhow::anyhow!("serde_json error, text = {}", text));
        }

        let mut result = result.unwrap();

        let mut retry_count = 0;
        let retry_max = 20;
        while result.output.is_none() {
            if retry_count == retry_max {
                return Err(anyhow::anyhow!("retry count full, replicate failed"));
            }

            time::sleep(Duration::from_secs(2)).await;

            let r = client
                .get(result.urls.get.clone())
                .headers(get_headers())
                .send()
                .await?;

            let status = r.status().as_u16();
            let text = r.text().await?;

            if (200..300).contains(&status) {
                let r = serde_json::from_str::<RespBody>(&text);

                if r.is_err() {
                    let text = format!("serde_json error, text = {}", text);
                    return Err(anyhow::anyhow!(text));
                }

                result = r.unwrap();

                if result.failed() {
                    return Err(anyhow::anyhow!("urls.get response with status failed"));
                }
            } else {
                let m = format!("urls.get request status: {} response: {}", status, text);
                return Err(anyhow::anyhow!("{}", m));
            }

            retry_count += 1;
        }

        let resp = client
            .get(result.output.as_ref().unwrap())
            .headers(get_headers())
            .send()
            .await?;

        if resp.status() == StatusCode::OK {
            Ok(resp.bytes().await?)
        } else {
            Err(anyhow::anyhow!("get file from replicate failed"))
        }
    } else {
        Err(anyhow::anyhow!(
            "post replicate status: {} response: {}",
            status,
            text
        ))
    }
}
#[derive(Serialize, Debug)]
struct ReqBody<'a> {
    version: &'a str,
    input: Input,
}
#[derive(Serialize, Debug)]
struct Input {
    image: String,
    scale: f32,
    face_enhance: bool,
}

impl ReqBody<'_> {
    pub fn new(image: &str, scale: f32) -> ReqBody<'static> {
        ReqBody {
            version: MODEL_VERSION.get_or_init(get_model_version),
            input: Input {
                image: image.to_string(),
                scale,
                face_enhance: false,
            },
        }
    }
}

#[derive(Deserialize)]
struct RespBody {
    // id: String,
    output: Option<String>,
    status: String,
    urls: Urls,
}

impl RespBody {
    pub fn failed(&self) -> bool {
        self.status == "failed"
    }
    // pub fn success(&self) -> bool {
    //     self.status.starts_with("success")
    // }
}

#[derive(Deserialize)]
struct Urls {
    get: String,
}
