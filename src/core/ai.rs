use std::sync::OnceLock;
use std::{collections::HashMap, env};

use anyhow::{Ok, Result};

use bytes::Bytes;
use reqwest::header::HeaderMap;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use tracing::error;

static CLIENT: OnceLock<Client> = OnceLock::new();
static MODEL_VERSION: OnceLock<String> = OnceLock::new();

fn get_client() -> Client {
    let mut c = reqwest::ClientBuilder::new();

    #[cfg(not(feature = "use-proxy"))]
    {
        c = c.no_proxy();
    }

    c.build().unwrap()
}

fn get_headers() -> HeaderMap {
    let token = env::var("REPLICATE_API_TOKEN").unwrap();
    let mut headers = HashMap::with_capacity(3);
    headers.insert("Authorization".to_string(), format!("Bearer {}", token));
    headers.insert("content-type".to_string(), "application/json".to_string());
    headers.insert("Prefer".to_string(), "wait".to_string());

    return (&headers).try_into().unwrap();
}

fn get_model_version() -> String {
    env::var("REPLICATE_MODEL_VERSION").unwrap()
}

pub struct AiScaleUp;

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

impl AiScaleUp {
    pub async fn resize(self, img_src: &str, scale_factor: f32) -> Result<Bytes> {
        let client = CLIENT.get_or_init(|| get_client());
        let body = ReqBody::new(img_src, scale_factor);
        let r = client
            .post("https://api.replicate.com/v1/predictions")
            .headers(get_headers())
            .body(serde_json::to_string(&body)?);

        let r = r.send().await?;
        let status = r.status().as_u16();
        let text = r.text().await?;

        if status < 300 && status >= 200 {
            let result = serde_json::from_str::<RespBody>(&text);

            if result.is_err() {
                let text = format!("serde_json error, text = {}", text);
                error!(text);
                return Err(anyhow::anyhow!(text));
            }

            let resp = client.get(result.unwrap().output).send().await?;

            if resp.status() == StatusCode::OK {
                return Ok(resp.bytes().await?);
            } else {
                return Err(anyhow::anyhow!("get file from replicate failed"));
            }
        } else {
            error!("replicate status: {}", status);
            error!("replicate response: {}", text);
            return Err(anyhow::anyhow!("get file from replicate failed"));
        }
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
            version: MODEL_VERSION.get_or_init(|| get_model_version()),
            input: Input {
                image: image.to_string(),
                scale,
                face_enhance: false,
            },
        }
    }
}

#[derive(Deserialize, Debug)]
struct RespBody {
    id: String,
    output: String,
}
