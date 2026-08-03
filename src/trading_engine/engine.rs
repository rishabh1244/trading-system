use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::cmp;
use std::sync::Mutex;
use std::task::Wake;

use domain::common::Balances;
use domain::trades::TradeList;

pub async fn update_balance(
    user_id: i32,
    trade: TradeList,
    pool: &PgPool,
) -> Result<Balances, sqlx::Error> {
    let mut user_balance: Balances = sqlx::query_as::<_, Balances>(
        "SELECT user_id, balance_btc::float8, balance_inr::float8 from balances where user_id=$1",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    for trade in trade.trades.iter() {
        if trade.trader_id == user_id {
            // user is the buyer: receives BTC, pays INR
            user_balance.balance_btc += trade.qty as f64;
            user_balance.balance_inr -= (trade.qty * trade.price) as f64;
        } else if trade.seller_id == user_id {
            // user is the seller: gives up BTC, receives INR
            user_balance.balance_btc -= trade.qty as f64;
            user_balance.balance_inr += (trade.qty * trade.price) as f64;
        }
    }

    sqlx::query("UPDATE balances SET balance_btc=$1, balance_inr=$2 WHERE user_id=$3")
        .bind(user_balance.balance_btc)
        .bind(user_balance.balance_inr)
        .bind(user_id)
        .execute(pool)
        .await?;

    Ok(user_balance)
}
