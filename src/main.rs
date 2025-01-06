mod api;
mod auth;
mod core;
mod db;

use api::resize::{resize, resize_free};
use auth::Auth;
use poem::middleware::{CatchPanic, Cors};
use poem::EndpointExt;
use poem::{get, handler, listener::TcpListener, post, IntoResponse, Result, Route, Server};
#[handler]
fn helloworld() -> impl IntoResponse {
    "hello world".into_response()
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
        .at("/resizefree", post(resize_free))
        .at("/resize", post(resize).with(Auth))
        .with(Cors::new())
        .with(CatchPanic::new());

    Server::new(TcpListener::bind("0.0.0.0:53768"))
        .run(app)
        .await
}
