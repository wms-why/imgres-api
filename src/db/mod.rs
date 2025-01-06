use std::env;

use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    ConnectOptions, PgPool,
};
use tokio::sync::OnceCell;
use url::Url;
pub mod file;
pub mod user;
pub mod user_opt;
pub mod user_recharge;

static POOL: OnceCell<PgPool> = OnceCell::const_new();

pub async fn get_pool() -> &'static PgPool {
    POOL.get_or_init(async || {
        let url: String = env::var("DATABASE_URL").unwrap();
        let url = Url::parse(&url).unwrap();
        let connection_options = PgConnectOptions::from_url(&url).unwrap();

        PgPoolOptions::new()
            .max_connections(10)
            .connect_with(connection_options)
            .await
            .unwrap()
    })
    .await
}
