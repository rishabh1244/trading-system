use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::cmp;
use std::sync::Mutex;
use std::task::Wake;

//update the user's asset after placing the order;
pub fn updateAsset(user_id: i32, btc_balance: i32, inr_balance: i32, pool: &PgPool) {
    let result = sqlx::query("UPDATE balances SET balance_btc=$1 balance_inr=$2 WHERE user_id=$3")
        .bind(btc_balance)
        .bind(inr_balance)
        .bind(user_id)
        .execute(pool)
        .await;
}
