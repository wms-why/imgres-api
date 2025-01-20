mod api;
mod auth;
mod core;
mod db;

use api::resize::{resize, resize_free};
use auth::Auth;
use poem::{
    get, handler, listener::TcpListener, middleware::CatchPanic, post, EndpointExt, IntoResponse,
    Result, Route, Server,
};
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

    let app = Route::new()
        .at("/hello", get(helloworld))
        .at("/api/resizefree", post(resize_free))
        // .at("/api/resize", post(resize).with(Auth))
        .at("/api/resize", post(resize))
        .with(CatchPanic::new());

    Server::new(TcpListener::bind("0.0.0.0:3001"))
        .run(app)
        .await
}
