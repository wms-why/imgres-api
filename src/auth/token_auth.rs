use google_oauth::GooglePayload;
use poem::{
    web::headers::{authorization::Bearer, Authorization, HeaderMapExt},
    Endpoint, IntoResponse, Request, Response, Result,
};
use std::sync::Arc;
use tracing::{debug, error};
use anyhow;

use crate::db::user::{self, Model};

pub struct TokenAuth<E>(pub E);

impl<E: Endpoint> Endpoint for TokenAuth<E> {
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        let authorization = req.headers().typed_get::<Authorization<Bearer>>();

        if authorization.is_some() {
            
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

async fn set_current_user(req: &mut Request, payload: GooglePayload) -> anyhow::Result<()> {
    debug!(
        "GooglePayload: {}",
        serde_json::to_string(&payload)?
    );

    let email = payload.email.clone().unwrap();
    let u = user::get_by_email(email.as_ref()).await?;

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
    anyhow::Ok(())
}

pub fn get_current_user(req: &Request) -> Option<&Model> {
    let u: Option<&Arc<Model>> = req.extensions().get();

    u?;

    let u = u.unwrap();

    Some(u.as_ref())
}
