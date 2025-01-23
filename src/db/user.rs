use super::get_pool;

use anyhow;
use chrono::{NaiveDateTime, Utc};
use sea_orm::{entity::prelude::*, Set};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Deserialize, Serialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
    pub email: String,
    pub credit: i64,
    pub ctime: NaiveDateTime,
    pub reg_from: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl ActiveModel {
    pub fn new_from(name: &str, email: &str) -> Self {
        ActiveModel {
            id: Set(0i32),
            name: Set(name.to_string()),
            email: Set(email.to_string()),
            credit: Set(10),
            ctime: Set(Utc::now().naive_utc()),
            reg_from: Set("google".to_string()),
        }
    }
}

pub async fn get_by_email(email: &str) -> anyhow::Result<Option<Model>> {
    let conn = get_pool().await;

    let user = Entity::find()
        .filter(Column::Email.eq(email))
        .one(conn)
        .await?;

    Ok(user)
}

pub async fn insert(name: &str, email: &str) -> anyhow::Result<Model> {
    let u = ActiveModel::new_from(name, email);
    let conn = get_pool().await;

    let user = u.insert(conn).await;

    Ok(user?)
}

pub async fn get_by_id(id: i32) -> anyhow::Result<Option<Model>> {
    let conn = get_pool().await;

    let user = Entity::find_by_id(id).one(conn).await?;

    Ok(user)
}

pub async fn update_credits(user: Model, credits_delta: i64) -> anyhow::Result<()> {
    let conn = get_pool().await;

    // Into ActiveModel
    let mut user: ActiveModel = user.into();

    // Update name attribute
    user.credit = Set(user.credit.unwrap() + credits_delta);

    // SQL: `UPDATE "fruit" SET "name" = 'Sweet pear' WHERE "id" = 28`
    user.update(conn).await?;

    Ok(())
}
