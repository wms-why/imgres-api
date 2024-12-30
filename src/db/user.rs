use std::rc::Rc;

use super::get_pool;
use anyhow::Ok;
use sqlx::types::chrono;

use anyhow::Result;

#[derive(Debug, Clone, sqlx::Type)]
#[sqlx(type_name = "reg_from", rename_all = "lowercase")]
pub enum RegFrom {
    Google,
    Other,
}

#[derive(sqlx::FromRow, Clone)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub credit: i64,
    pub ctime: chrono::NaiveDateTime,
    pub reg_from: RegFrom,
}

impl User {
    pub fn new(name: &str, email: &str) -> User {
        User {
            id: 0,
            name: name.to_string(),
            email: email.to_string(),
            credit: 10,
            ctime: chrono::Utc::now().naive_utc(),
            reg_from: RegFrom::Google,
        }
    }
}

pub async fn get_by_email(email: &str) -> Option<User> {
    let conn = get_pool().await;

    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = ? limit 1")
        .bind(email)
        .fetch_one(conn)
        .await;

    if user.is_err() {
        return None;
    }

    Some(user.unwrap())
}

pub async fn insert(name: &str, email: &str) -> Result<User> {
    let mut u = User::new(name, email);
    let conn = get_pool().await;

    let row: (i32,) = sqlx::query_as("INSERT INTO users (name, email, credit, ctime, reg_from) VALUES (?, ?, ?, ?, ?) RETURNING id")
        .bind(u.name.clone())
        .bind(u.email.clone())
        .bind(u.credit)
        .bind(u.ctime)
        .bind(u.reg_from.clone())
        .fetch_one(conn)
        .await?;
    u.id = row.0;

    Ok(u)
}
