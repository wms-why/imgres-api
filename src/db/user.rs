use std::hash::{DefaultHasher, Hash, Hasher};

use super::kv::{self, KvReqBody};

use anyhow::{self};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct User {
    pub email: String,
    pub name: String,
    pub credit: i64,
    pub ctime: DateTime<Local>,
    pub reg_from: String,
}

fn gen_key(email: &str) -> String {
    let mut s = DefaultHasher::new();
    email.hash(&mut s);
    format!("user_{}", s.finish())
}

impl User {
    pub fn new(name: &str, email: &str) -> Self {
        User {
            email: email.to_string(),
            name: name.to_string(),
            credit: 10,
            ctime: Local::now(),
            reg_from: "google".to_string(),
        }
    }
}

pub async fn get_by_email(email: &str) -> anyhow::Result<Option<User>> {
    let key = gen_key(email);

    let user_info = kv::get(&key).await?;

    if user_info.is_none() {
        return Ok(None);
    }

    let user_info = user_info.unwrap();
    let user_info = serde_json::from_str::<User>(&user_info)?;

    Ok(Some(user_info))
}

pub async fn insert(name: &str, email: &str) -> anyhow::Result<User> {
    let user = User::new(name, email);

    let key = gen_key(email);

    let body = KvReqBody::new(key.clone(), serde_json::to_string(&user)?, None);
    kv::insert(&body).await?;

    Ok(user)
}

pub async fn update_credits(user: &mut User, credits_delta: i64) -> anyhow::Result<()> {
    user.credit += credits_delta;

    let key = gen_key(&user.email).to_string();

    let body = KvReqBody::new(key.clone(), serde_json::to_string(&user)?, None);
    kv::insert(&body).await?;

    anyhow::Ok(())
}
