use std::env;

use sea_orm::{Database, DatabaseConnection};
use tokio::sync::OnceCell;
use url::Url;
pub mod file;
pub mod user;
pub mod user_opt;
pub mod user_recharge;

static POOL: OnceCell<DatabaseConnection> = OnceCell::const_new();

pub async fn get_pool() -> &'static DatabaseConnection {
    POOL.get_or_init(|| {
        Box::pin(async {
            let url: String = env::var("DATABASE_URL").unwrap();
            let url = Url::parse(&url).unwrap();
            let db = Database::connect(url).await;
            db.unwrap()
        })
    })
    .await
}
