use std::{env, sync::OnceLock};

use google_oauth::{AsyncClient, GooglePayload};
use poem::{
    handler,
    http::StatusCode,
    web::{Json, Query},
    IntoResponse, Response,
};
use serde::Serialize;
use tracing::error;

use crate::db::user;

#[derive(Serialize)]
struct LoginResp {
    username: String,
    token: String,

    // 秒
    exp: i64,
}

impl LoginResp {
    fn new(username: String, token: String) -> Self {
        // 当前秒数
        let exp = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // 24h
        let exp_duration = 60 * 60 * 24;
        Self {
            username,
            token,
            exp: exp + exp_duration,
        }
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

    if !validate_payload(&r) {
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

        if (insert_r.is_err()) {
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

    let resp = LoginResp::new(name, token);

    return Json(resp).into_response();
}

fn validate_payload(payload: &GooglePayload) -> bool {
    payload.email.is_some() && payload.name.is_some()
}
