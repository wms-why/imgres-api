use google_oauth::{AsyncClient, GooglePayload};
use poem::{
    http::StatusCode,
    web::headers::{authorization::Bearer, Authorization, HeaderMapExt},
    Endpoint, IntoResponse, Request, Response, Result,
};
use std::{env, sync::Arc, sync::OnceLock};
use tracing::{debug, error};

use crate::db::user::{self, User};

static CLIENT: OnceLock<AsyncClient> = OnceLock::new();

fn get_client() -> &'static AsyncClient {
    let client_id = env::var("GOOGLE_CLIENT_ID").expect("GOOGLE_CLIENT_ID is not set");
    CLIENT.get_or_init(|| AsyncClient::new(client_id))
}

pub struct TokenAuth<E>(pub E);

impl<E: Endpoint> Endpoint for TokenAuth<E> {
    type Output = Response;

    async fn call(&self, mut req: Request) -> Result<Self::Output> {
        let authorization = req.headers().typed_get::<Authorization<Bearer>>();

        if authorization.is_some() {
            let r = get_client()
                .validate_id_token(authorization.unwrap().token())
                .await;
            if r.is_err() {
                error!("google validate_id_token error: {}", r.err().unwrap());
                return Ok(Response::builder()
                    .status(StatusCode::UNAUTHORIZED)
                    .finish());
            }

            let r = r.unwrap();

            if !validate_payload(&r) {
                error!("google GooglePayload validate error: {:?}", r);
                return Ok(Response::builder()
                    .status(StatusCode::UNAUTHORIZED)
                    .finish());
            }
            if let Err(e) = set_current_user(&mut req, r).await {
                error!("set_current_user error: {}", e);
                return Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .finish());
            }
        }

        let res = self.0.call(req).await;
        match res {
            Ok(resp) => {
                let resp = resp.into_response();
                debug!("response: {}", resp.status());
                Ok(resp)
            }
            Err(err) => {
                error!("error: {err}");
                Err(err)
            }
        }
    }
}

fn validate_payload(payload: &GooglePayload) -> bool {
    payload.email.is_some()
}

async fn set_current_user(req: &mut Request, payload: GooglePayload) -> anyhow::Result<()> {
    debug!(
        "GooglePayload: {}",
        serde_json::to_string(&payload).unwrap()
    );

    let email = payload.email.clone().unwrap();
    let u = user::get_by_email(email.as_ref()).await;

    if let Some(u) = u {
        req.set_data(Arc::new(u));
    } else {
        let mut name = payload.name;

        if name.is_none() {
            name = Some("".to_string());
        }
        let uu = user::insert(name.unwrap().as_str(), &email).await?;
        let uu = Arc::new(uu);
        req.set_data(uu);
    }
    Ok(())
}

pub fn get_current_user(req: &Request) -> Option<&User> {
    let u: Option<&Arc<User>> = req.extensions().get();

    u?;

    let u = u.unwrap();

    Some(u.as_ref())
}
