mod api;
mod auth;
mod core;
mod db;

use api::resize_api::resize;
use auth::Auth;
use poem::middleware::Cors;
use poem::EndpointExt;
use poem::{
    get, handler,
    listener::TcpListener,
    post,
    session::{CookieConfig, CookieSession},
    IntoResponse, Result, Route, Server,
};
#[handler]
fn helloworld() -> impl IntoResponse {
    return "hello world".into_response();
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    dotenvy::dotenv().expect("Failed to load .env file");

    if std::env::var_os("RUST_LOG").is_none() {
        unsafe {
            std::env::set_var("RUST_LOG", "INFO");
        }
    }
    tracing_subscriber::fmt::init();

    // Cors::new().allow_origin(origin)

    let app = Route::new()
        .at("/hello", get(helloworld))
        .at("/resize", post(resize).with(Auth))
        .with(CookieSession::new(CookieConfig::new()))
        .with(Cors::new());

    Server::new(TcpListener::bind("0.0.0.0:53768"))
        .run(app)
        .await
}
