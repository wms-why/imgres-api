pub mod login;
pub mod resize;

mod params;

use poem::{http::StatusCode, Response};
fn gen_known_err_response(msg: &str) -> Response {
    Response::builder()
        .status(StatusCode::UNAVAILABLE_FOR_LEGAL_REASONS)
        .body(msg.to_string())
}
