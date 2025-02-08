use poem::{http::StatusCode, Error, FromRequest, Request, RequestBody, Result};
use tracing::debug;

use crate::{db::user, middleware::auth::get_auth_claims};

pub struct AuthUser {
    pub user: user::User,
}

// Implements a token extractor
impl<'a> FromRequest<'a> for AuthUser {
    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> Result<Self> {
        let claims = get_auth_claims(req);

        if claims.is_none() {
            return Err(Error::from_string(
                "token is invalid".to_string(),
                StatusCode::UNAUTHORIZED,
            ));
        }

        let c = claims.unwrap();

        debug!("claims: {:?}", c);

        let u = user::get_by_email(&c.email).await;

        debug!("user: {:?}", u);

        match u {
            Ok(Some(user_model)) => Ok(AuthUser { user: user_model }),
            Ok(None) => Err(Error::from_string(
                "user not found".to_string(),
                StatusCode::UNAUTHORIZED,
            )),
            _ => Err(Error::from_string(
                "email invalid or db connection error".to_string(),
                StatusCode::BAD_REQUEST,
            )),
        }
    }
}
