use chrono::NaiveDateTime;


pub struct UserOpt {
    pub id: i32,
    pub user_id: i32,
    pub ctime: NaiveDateTime,
    pub opts: String,
    pub cost_credits: i64,
}

pub fn insert(opt: &UserOpt) {}
