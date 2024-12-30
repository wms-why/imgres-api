use poem::{http::StatusCode, IntoResponse, Response};
pub mod resize;

pub fn check_login_error() -> impl IntoResponse {
    Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .finish()
}
