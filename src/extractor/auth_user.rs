use poem::{http::StatusCode, Error, FromRequest, Request, RequestBody, Result};

use crate::db::user;

pub struct AuthUser(user::Model);

// Implements a token extractor
impl<'a> FromRequest<'a> for AuthUser {
    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> Result<Self> {
        let token = req
            .headers()
            .get("MyToken")
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| Error::from_string("missing token", StatusCode::BAD_REQUEST))?;
        Ok(AuthUser(token.to_string()))
    }
}
