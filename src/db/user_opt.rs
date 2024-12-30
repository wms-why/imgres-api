use sqlx::types::chrono;

#[derive(sqlx::FromRow, Default)]
pub struct UserOpt {
    pub id: i32,
    pub user_id: i32,
    pub ctime: chrono::NaiveDateTime,
    pub opts: String,
    pub cost_credits: i64,
}

pub fn insert(opt: &UserOpt) {}
