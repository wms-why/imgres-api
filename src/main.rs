mod api;
mod auth;
mod core;
mod db;

use std::io::Read;

use api::resize::{resize, resize_free};
use auth::Auth;
use poem::{
    get, handler,
    listener::{Listener, RustlsCertificate, RustlsConfig, TcpListener},
    middleware::{CatchPanic, Cors},
    post, EndpointExt, IntoResponse, Result, Route, Server,
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

    // Cors::new().allow_origin(origin)

    let app = Route::new()
        .at("/hello", get(helloworld))
        .at("/resizefree", post(resize_free))
        .at("/resize", post(resize).with(Auth))
        .with(Cors::new())
        .with(CatchPanic::new());

    if std::env::var("PROFILE").is_ok_and(|v| v == "dev") {
        let listener = TcpListener::bind("0.0.0.0:53768");

        Server::new(listener).run(app).await
    } else {
        let cert = std::fs::File::open("cert.pem").expect("Failed to get cert.pem");
        let key = std::fs::File::open("key.pem").expect("Failed to get key.pem");

        let cert = std::io::BufReader::new(cert);
        let key = std::io::BufReader::new(key);

        let mut cert_str = "".to_string();
        let size = cert
            .read_to_string(&mut cert_str)
            .expect("Failed to read cert.pem");

        if size == 0 {
            panic!("cert.pem is blank");
        }

        let mut key_str = "".to_string().expect("Failed to read key.pem");
        let size = key.read_to_string(&mut key_str);

        if size == 0 {
            panic!("key.pem is blank");
        }

        let listener = TcpListener::bind("0.0.0.0:53768").rustls(
            RustlsConfig::new().fallback(RustlsCertificate::new().key(key_str).cert(cert_str)),
        );
        Server::new(listener).run(app).await
    }
}
