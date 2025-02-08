use poem::{
    web::headers::{authorization::Bearer, Authorization, HeaderMapExt},
    Endpoint, IntoResponse, Request, Response, Result,
};
use tracing::{debug, error};

use crate::api::login::{decode_from_token, Claims};

use super::check_login_error;

pub struct AuthorizationCheck<E>(pub E);

impl<E: Endpoint> Endpoint for AuthorizationCheck<E> {
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        let claims = get_auth_claims(&req);
        if claims.is_none() {
            return Ok(check_login_error());
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

// 无io消耗
pub fn get_auth_claims(req: &Request) -> Option<Box<Claims>> {
    let authorization = req.headers().typed_get::<Authorization<Bearer>>();

    if authorization.is_none() {
        return None;
    }

    let authorization = authorization.unwrap();

    let claims = decode_from_token(authorization.token());
    if claims.is_err() {
        return None;
    }

    let claims = claims.unwrap();

    Some(Box::new(claims))
}
