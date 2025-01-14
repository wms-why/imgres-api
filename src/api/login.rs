use std::{env, sync::OnceLock};

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

use crate::db::user;

#[derive(Serialize, Deserialize)]
struct Claims {
    user_id: i32,
    username: String,
    // 秒
    exp: i64,
}
impl Claims {
    fn new(user_id: i32, username: String) -> Self {
        // 当前秒数
        let exp = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // 24h
        let exp_duration = 60 * 60 * 24;
        Self {
            user_id,
            username,
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
    meta: Claims,
}

impl From<Claims> for LoginResp {
    fn from(value: Claims) -> Self {
        let secret = get_token_secret().as_bytes();

        let token = encode(
            &Header::default(),
            &value,
            &EncodingKey::from_secret(secret),
        )
        .unwrap();

        Self { token, meta: value }
    }
}

static CLIENT: OnceLock<AsyncClient> = OnceLock::new();

fn get_client() -> &'static AsyncClient {
    let client_id = env::var("GOOGLE_CLIENT_ID").expect("GOOGLE_CLIENT_ID is not set");
    CLIENT.get_or_init(|| AsyncClient::new(client_id))
}

#[handler]
pub async fn login(Query(token): Query<String>) -> Response {
    let r = get_client().validate_id_token(&token).await;
    if r.is_err() {
        error!("google validate_id_token error: {}", r.err().unwrap());
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .finish();
    }

    let r = r.unwrap();

    if !validate_google_payload(&r) {
        error!("google GooglePayload validate error: {:?}", r);
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .finish();
    }

    let email = r.email.unwrap();
    let name = r.name.unwrap();

    let u = user::get_by_email(&email).await;

    if u.is_err() {
        error!("get user by email error: {}", u.err().unwrap());
        return Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .finish();
    }

    let u = u.unwrap();

    let mut user;
    if u.is_none() {
        let insert_r = user::insert(name.as_ref(), email.as_ref()).await;

        if insert_r.is_err() {
            error!("insert user error: {}", insert_r.err().unwrap());
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .finish();
        } else {
            user = insert_r.unwrap();
        }
    } else {
        user = u.unwrap();
    }

    let meta = Claims::new(user.id, name);

    Json(LoginResp::from(meta)).into_response()
}

fn validate_google_payload(payload: &GooglePayload) -> bool {
    payload.email.is_some() && payload.name.is_some()
}

pub fn decode_from_token(
    token: &str,
) -> Result<jsonwebtoken::TokenData<Claims>, jsonwebtoken::errors::Error> {
    let secret = get_token_secret().as_bytes();

    let mut v = Validation::default();
    v.validate_aud = false;
    decode::<Claims>(token, &DecodingKey::from_secret(secret), &v)
}
