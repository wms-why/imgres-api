use poem::{http::StatusCode, Error, FromRequest, Request, RequestBody, Result};

use crate::{api::login::Claims, db::user, middleware::auth::get_auth_claims};

pub struct AuthUser {
    pub user: user::Model,
}

// Implements a token extractor
impl<'a> FromRequest<'a> for AuthUser {
    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> Result<Self> {
        let claims: Option<&Claims> = get_auth_claims(req);

        if claims.is_none() {
            return Err(Error::from_string(
                "token is invalid".to_string(),
                StatusCode::UNAUTHORIZED,
            ));
        }

        let c = claims.unwrap();

        let u = user::get_by_id(c.user_id).await;

        match u {
            Ok(Some(user_model)) => Ok(AuthUser { user: user_model }),
            _ => Err(Error::from_string(
                "user id invalid or db connection error".to_string(),
                StatusCode::BAD_REQUEST,
            )),
        }
    }
}
