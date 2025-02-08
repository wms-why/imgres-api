use std::{collections::HashMap, env, time::Duration};

use reqwest::{header::HeaderMap, Client};
use serde::{Deserialize, Serialize};
use tokio::sync::OnceCell;

struct KvConfig {
    token: String,
    namespace_id: String,
    account_id: String,
}

static KV_CONFIG: OnceCell<KvConfig> = OnceCell::const_new();
static CLIENT: OnceCell<reqwest::Client> = OnceCell::const_new();

async fn get_kv_config() -> &'static KvConfig {
    KV_CONFIG
        .get_or_init(|| {
            Box::pin(async {
                let token = env::var("CF_KV_TOKEN").unwrap();
                let namespace_id = env::var("CF_KV_NAMESPACE_ID").unwrap();
                let account_id = env::var("CF_KV_ACCOUNT_ID").unwrap();
                KvConfig {
                    token,
                    namespace_id,
                    account_id,
                }
            })
        })
        .await
}

async fn get_client() -> &'static Client {
    CLIENT
        .get_or_init(|| {
            Box::pin(async {
                reqwest::ClientBuilder::new()
                    .default_headers(get_headers().await)
                    .connect_timeout(Duration::from_secs(3))
                    .timeout(Duration::from_secs(10))
                    .no_proxy()
                    .build()
                    .unwrap()
            })
        })
        .await
}

pub async fn insert_batch(list: &Vec<KvReqBody>) -> anyhow::Result<()> {
    let reqwest_client = get_client().await;

    let config = get_kv_config().await;

    let response = reqwest_client
        .put(format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/storage/kv/namespaces/{}/bulk",
            &config.account_id, &config.namespace_id
        ))
        .body(serde_json::to_string(list).unwrap())
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status() == reqwest::StatusCode::OK {
                let body = resp.json::<KvRespBody>().await?;

                if body.success {
                    anyhow::Ok(())
                } else {
                    Err(anyhow::anyhow!("insert_batch resp error: {:?}", body))
                }
            } else {
                Err(anyhow::anyhow!(
                    "insert_batch resp error: status_code={:?},msg={}",
                    resp.status(),
                    resp.text().await?
                ))
            }
        }
        Err(e) => Err(anyhow::anyhow!("insert_batch req error: {:?}", e)),
    }
}

pub async fn insert(item: &KvReqBody) -> anyhow::Result<()> {
    let reqwest_client = get_client().await;

    let config = get_kv_config().await;

    let url = if item.expiration_ttl.is_some() {
        format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/storage/kv/namespaces/{}/values/{}?expiration_ttl={}",
            &config.account_id, &config.namespace_id, item.key, item.expiration_ttl.unwrap()
        )
    } else {
        format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/storage/kv/namespaces/{}/values/{}",
            &config.account_id, &config.namespace_id, item.key
        )
    };

    let response = reqwest_client
        .put(&url)
        .body(item.value.clone())
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status() == reqwest::StatusCode::OK {
                let body = resp.json::<KvRespBody>().await?;

                if body.success {
                    anyhow::Ok(())
                } else {
                    Err(anyhow::anyhow!("insert resp error: {:?}", body))
                }
            } else {
                Err(anyhow::anyhow!(
                    "insert http resp error: status_code={:?}, msg={}",
                    resp.status(),
                    resp.text().await?
                ))
            }
        }
        Err(e) => Err(anyhow::anyhow!("insert req error: {:?}", e)),
    }
}

pub async fn get(key: &str) -> anyhow::Result<Option<String>> {
    let reqwest_client = get_client().await;

    let config = get_kv_config().await;

    let response = reqwest_client
        .get(format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/storage/kv/namespaces/{}/values/{}",
            &config.account_id, &config.namespace_id, key
        ))
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status() == reqwest::StatusCode::OK {
                anyhow::Ok(Some(resp.text().await?))
            } else if resp.status() == reqwest::StatusCode::NOT_FOUND {
                anyhow::Ok(None)
            } else {
                Err(anyhow::anyhow!(
                    "get_by_key resp error: status_code={:?}, msg={}",
                    resp.status(),
                    resp.text().await?
                ))
            }
        }
        Err(e) => Err(anyhow::anyhow!("get_by_key req error: {:?}", e)),
    }
}
async fn get_headers() -> HeaderMap {
    let config = get_kv_config().await;

    let mut headers = HashMap::with_capacity(2);
    headers.insert(
        "Authorization".to_string(),
        format!("Bearer {}", &config.token),
    );
    headers.insert("content-type".to_string(), "application/json".to_string());

    (&headers).try_into().expect("valid headers")
}

#[derive(Serialize, Debug)]
pub struct KvReqBody {
    base64: bool,

    expiration_ttl: Option<i32>,

    key: String,

    value: String,
}

impl KvReqBody {
    pub fn new(key: String, value: String, expiration_ttl: Option<i32>) -> KvReqBody {
        KvReqBody {
            base64: false,
            expiration_ttl,
            key,
            value,
        }
    }
}

#[derive(Serialize, Debug, Deserialize)]
struct KvRespBody {
    errors: Vec<KvRespItem>,
    messages: Vec<KvRespItem>,
    success: bool,
}

#[derive(Serialize, Debug, Deserialize)]
struct KvRespItem {
    code: i32,
    message: String,
}
