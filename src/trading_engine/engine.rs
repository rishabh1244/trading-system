use crate::domain::common::Balances;
use crate::domain::trades::TradeList;
use sqlx::PgPool;
use std::collections::{HashMap, HashSet};

pub async fn settle_trades(
    user_id: i32,
    trade: TradeList,
    pool: &PgPool,
) -> Result<Balances, sqlx::Error> {
    let mut involved: HashSet<i32> = HashSet::new();
    involved.insert(user_id);
    for trade in trade.trades.iter() {
        involved.insert(trade.trader_id);
        involved.insert(trade.seller_id);
    }

    let mut balances: HashMap<i32, Balances> = HashMap::new();
    for id in involved {
        let balance: Balances = sqlx::query_as::<_, Balances>(
            "SELECT user_id, balance_btc::float8, balance_inr::float8 from balances where user_id=$1",
        )
         .bind(id)
        .fetch_one(pool)
        .await?;
        balances.insert(id, balance);
    }

    for trade in trade.trades.iter() {
        // buyer: receives BTC, pays INR
        let buyer = balances.get_mut(&trade.trader_id).unwrap();
        buyer.balance_btc += trade.qty as f64;
        buyer.balance_inr -= (trade.qty * trade.price) as f64;

        // seller: gives up BTC, receives INR
        let seller = balances.get_mut(&trade.seller_id).unwrap();
        seller.balance_btc -= trade.qty as f64;
        seller.balance_inr += (trade.qty * trade.price) as f64;
    }

    for (id, balance) in balances.iter() {
        sqlx::query("UPDATE balances SET balance_btc=$1, balance_inr=$2 WHERE user_id=$3")
            .bind(balance.balance_btc)
            .bind(balance.balance_inr)
            .bind(id)
            .execute(pool)
            .await?;
    }

    Ok(balances[&user_id].clone())
}

pub async fn update_balance(
    user_id: i32,
    side: &str,
    qty: i32,
    price: i32,
    pool: &PgPool,
) -> Result<Balances, sqlx::Error> {
    let mut balance: Balances = sqlx::query_as::<_, Balances>(
        "SELECT user_id, balance_btc::float8, balance_inr::float8 from balances where user_id=$1",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    if side == "BUY" {
        // lock INR to fund the buy order
        balance.balance_inr -= (qty * price) as f64;
    } else if side == "SELL" {
        // lock BTC to fund the sell order
        balance.balance_btc -= qty as f64;
    }

    sqlx::query("UPDATE balances SET balance_btc=$1, balance_inr=$2 WHERE user_id=$3")
        .bind(balance.balance_btc)
        .bind(balance.balance_inr)
        .bind(user_id)
        .execute(pool)
        .await?;

    Ok(balance)
}

// remaining functionality :-
//
//  - update the balances of user's who have sold there assets (currently only updating the user who
//  have sent the request for buy/sell)
//  - update the remaining balance of the user after all the possible trades are fullfiled
//  - presist the orderbook in the database
//
