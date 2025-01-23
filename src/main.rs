mod api;
mod core;
mod db;
mod extractor;
mod middleware;

use api::{
    login::login,
    resize::{resize, resize_free},
};
use middleware::{auth::get_auth_claims, AuthMiddleware};
use poem::{
    get, handler, listener::TcpListener, middleware::CatchPanic, post, EndpointExt, IntoResponse,
    Request, Result, Route, Server,
};
#[handler]
fn helloworld(req: &Request) -> impl IntoResponse {
    let u = get_auth_claims(req);

    if let Some(u) = u {
        println!("user: {:?}", u);
    }
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
        .at("/api/hello", get(helloworld))
        .at("/api/resizefree", post(resize_free))
        .at("/api/resize", post(resize).with(AuthMiddleware))
        // .at("/api/resize", post(resize))
        .at("/api/login", get(login))
        .with(CatchPanic::new());

    Server::new(TcpListener::bind("0.0.0.0:3001"))
        .run(app)
        .await
}
