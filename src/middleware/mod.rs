pub mod auth;

use poem::{http::StatusCode, Endpoint, Middleware, Response};

pub struct AuthMiddleware;

impl<E: Endpoint> Middleware<E> for AuthMiddleware {
    type Output = auth::AuthorizationCheck<E>;

    fn transform(&self, ep: E) -> Self::Output {
        auth::AuthorizationCheck(ep)
    }
}

fn check_login_error() -> Response {
    Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .finish()
}
