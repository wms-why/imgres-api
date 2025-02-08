use std::{env, sync::OnceLock};

use crate::db::user::{self, User};
use anyhow::Result;
use google_oauth::{AsyncClient, GooglePayload};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use poem::{
    handler,
    http::StatusCode,
    web::{Json, Query},
    IntoResponse, Response,
};
use serde::{Deserialize, Serialize};
use tracing::error;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Claims {
    pub username: String,
    pub email: String,
    pub picture: Option<String>,
    // 秒
    pub exp: i64,
}
impl Claims {
    pub fn new(username: String, email: String, picture: Option<String>) -> Self {
        // 当前秒数
        let exp = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // 24h
        let exp_duration = 60 * 60 * 24;
        Self {
            username,
            picture,
            email,
            exp: exp + exp_duration,
        }
    }
}

static TOKEN_SECRET: OnceLock<String> = OnceLock::new();
fn get_token_secret() -> &'static String {
    TOKEN_SECRET.get_or_init(|| env::var("TOKEN_SECRET").expect("TOKEN_SECRET is not set"))
}

#[derive(Serialize)]
struct LoginResp {
    token: String,
    meta: UserMeta,
}

#[derive(Serialize)]
struct UserMeta {
    pub username: String,
    pub email: String,
    pub picture: Option<String>,
    pub credits: i64,
    // 秒
    exp: i64,
}

impl UserMeta {
    fn from(value: &Claims, user: &User) -> Self {
        Self {
            username: value.username.clone(),
            email: value.email.clone(),
            picture: value.picture.clone(),
            credits: user.credit,
            exp: value.exp,
        }
    }
}

impl LoginResp {
    fn from(value: &Claims, user: &User) -> Self {
        let secret = get_token_secret().as_bytes();

        let token = encode(
            &Header::default(),
            &value,
            &EncodingKey::from_secret(secret),
        )
        .unwrap();

        Self {
            token,
            meta: UserMeta::from(value, user),
        }
    }
}

pub fn decode_from_token(token: &str) -> Result<Claims> {
    let secret = get_token_secret().as_bytes();

    let mut v = Validation::default();
    v.validate_aud = false;
    let c = decode::<Claims>(token, &DecodingKey::from_secret(secret), &v)?;

    Ok(c.claims)
}

static CLIENT: OnceLock<AsyncClient> = OnceLock::new();

fn get_client() -> &'static AsyncClient {
    let client_id = env::var("GOOGLE_CLIENT_ID").expect("GOOGLE_CLIENT_ID is not set");
    CLIENT.get_or_init(|| AsyncClient::new(client_id))
}

#[derive(Deserialize)]
struct LoginParams {
    token: String,
}

#[handler]
pub async fn login(Query(LoginParams { token }): Query<LoginParams>) -> Response {
    let r = get_client().validate_id_token(&token).await;
    if r.is_err() {
        let body = format!("google validate_id_token error: {}", r.err().unwrap());
        error!(body);
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(body);
    }

    let r = r.unwrap();

    if let Err(e) = validate_google_payload(&r) {
        error!("google account validate error: {:?}", r);
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(e.to_string());
    }

    let email = r.email.unwrap();
    let name = r.name.unwrap();
    let picture = r.picture;

    let u: Result<Option<user::User>, anyhow::Error> = user::get_by_email(&email).await;

    if u.is_err() {
        error!("get user by email error: {}", u.err().unwrap());
        return Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body("db error");
    }

    let u = u.unwrap();

    let mut user;
    if u.is_none() {
        let insert_r = user::insert(name.as_ref(), email.as_ref()).await;

        if insert_r.is_err() {
            error!("insert user error: {}", insert_r.err().unwrap());
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body("db error");
        } else {
            user = insert_r.unwrap();
        }
    } else {
        user = u.unwrap();
    }

    let meta = Claims::new(name, email, picture);

    Json(LoginResp::from(&meta, &user)).into_response()
}

fn validate_google_payload(payload: &GooglePayload) -> Result<()> {
    if payload.email.is_some() && payload.name.is_some() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("google payload lack email or name"))
    }
}
