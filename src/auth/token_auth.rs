use poem::{
    web::headers::{authorization::Bearer, Authorization, HeaderMapExt},
    Endpoint, IntoResponse, Request, Response, Result,
};
use tracing::{debug, error};

use crate::{
    api::login::{decode_from_token, Claims},
    db::user,
};

use super::check_login_error;

pub struct TokenAuth<E>(pub E);

impl<E: Endpoint> Endpoint for TokenAuth<E> {
    type Output = Response;

    async fn call(&self, mut req: Request) -> Result<Self::Output> {
        let authorization = req.headers().typed_get::<Authorization<Bearer>>();

        if authorization.is_none() {
            return Ok(check_login_error());
        }

        let authorization = authorization.unwrap();

        let claims = decode_from_token(authorization.token());
        if claims.is_err() {
            return Ok(check_login_error());
        }

        let claims = claims.unwrap();

        set_auth_claims(&mut req, claims);

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

fn set_auth_claims(req: &mut Request, claims: Claims) {
    req.set_data(claims);
}

// 无io消耗
pub fn get_auth_claims(req: &Request) -> Option<&Claims> {
    let u: Option<&Claims> = req.extensions().get();

    u?;

    let u = u.unwrap();

    Some(u)
}


// 消耗一次db io
pub async fn get_user(req: &mut Request) -> Option<user::Model> {
    let u: Option<&user::Model> = req.extensions().get();

    if u.is_some() {
        return u.cloned();
    }

    let c = get_auth_claims(req)?;

    let u = user::get_by_id(c.user_id).await;

    match u {
        Ok(Some(user_model)) => {
            req.extensions_mut().insert(user_model.clone());
            Some(user_model)
        },
        _ => None,
    }
}
