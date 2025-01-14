use chrono::NaiveDateTime;


pub struct UserRecharge {
    pub id: i32,
    pub user_id: i32,
    pub amount: i64,
    pub ctime: NaiveDateTime,

    // 取消
    pub canceled: bool,

    // 退款
    pub refunded: bool,
}
