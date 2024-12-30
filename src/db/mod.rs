use std::env;

use aws_config::{BehaviorVersion, Region};
use aws_sdk_dsql::auth_token::{AuthTokenGenerator, Config};
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
        let db_type = env::var("DB_TYPE");

        if db_type.is_err() {
            get_supabase_pool().await
        } else if db_type.unwrap() == "aws" {
            get_aws_pool().await
        } else {
            get_supabase_pool().await
        }
    })
    .await
}

async fn get_aws_pool() -> PgPool {
    let region = env::var("AWS_REGION").unwrap();
    let aws_rdb_endpoint = env::var("AWS_RDB_ENDPOINT").unwrap();

    // Generate auth token
    let sdk_config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let signer = AuthTokenGenerator::new(
        Config::builder()
            .hostname(aws_rdb_endpoint.clone())
            .region(Region::new(region))
            .build()
            .unwrap(),
    );
    let password_token = signer
        .db_connect_admin_auth_token(&sdk_config)
        .await
        .unwrap();

    // Setup connections
    let connection_options = PgConnectOptions::new()
        .host(aws_rdb_endpoint.as_str())
        .port(5432)
        .database("postgres")
        .username("admin")
        .password(password_token.as_str())
        .ssl_mode(sqlx::postgres::PgSslMode::VerifyFull);

    PgPoolOptions::new()
        .max_connections(10)
        .connect_with(connection_options.clone())
        .await
        .unwrap()
}

async fn get_supabase_pool() -> PgPool {
    let url: String = env::var("DB_SUPABASE_URL").unwrap();
    let url = Url::parse(&url).unwrap();
    let connection_options = PgConnectOptions::from_url(&url).unwrap();

    PgPoolOptions::new()
        .max_connections(10)
        .connect_with(connection_options)
        .await
        .unwrap()
}
