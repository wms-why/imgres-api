pub mod token_auth;

use poem::{http::StatusCode, Endpoint, Middleware, Response};

pub struct Auth;

impl<E: Endpoint> Middleware<E> for Auth {
    type Output = token_auth::TokenAuth<E>;

    fn transform(&self, ep: E) -> Self::Output {
        token_auth::TokenAuth(ep)
    }
}

fn check_login_error() -> Response {
    Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .finish()
}
