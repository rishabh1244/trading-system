use crate::domain::common::Balances;
use crate::domain::order::Order;
use crate::domain::trades::TradeList;
use crate::trading_engine::orderbook::sync_orderbook;
use sqlx::PgPool;
use std::collections::{HashMap, HashSet};

pub async fn settle_trades(
    user_id: i32,
    trade: TradeList,
    appends: Option<Order>,
    fulfilled_ids: Vec<i32>,
    pool: &PgPool,
) -> Result<(Balances, Option<i32>), sqlx::Error> {
    let mut involved: HashSet<i32> = HashSet::new();
    involved.insert(user_id);
    for trade in trade.trades.iter() {
        involved.insert(trade.buyer_id);
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
        let qty = trade.qty as f64;
        let value = (trade.qty * trade.price) as f64;

        // buyer receives BTC, and pays INR if the aggressor is the buyer
        let buyer = balances.get_mut(&trade.buyer_id).unwrap();
        buyer.balance_btc += qty;
        if trade.buyer_id == user_id {
            buyer.balance_inr -= value;
        }
        drop(buyer);

        // seller receives INR, and gives up BTC if the aggressor is the seller
        let seller = balances.get_mut(&trade.seller_id).unwrap();
        seller.balance_inr += value;
        if trade.seller_id == user_id {
            seller.balance_btc -= qty;
        }
        drop(seller);
        // updates trades to datbase
        sqlx::query(
            "INSERT INTO trades (buyer_id , seller_id ,qty , price) VALUES ($1, $2, $3, $4)",
        )
        .bind(trade.buyer_id)
        .bind(trade.seller_id)
        .bind(trade.qty)
        .bind(trade.price)
        .execute(pool)
        .await?;
    }

    for (id, balance) in balances.iter() {
        sqlx::query("UPDATE balances SET balance_btc=$1, balance_inr=$2 WHERE user_id=$3")
            .bind(balance.balance_btc)
            .bind(balance.balance_inr)
            .bind(id)
            .execute(pool)
            .await?;
    }

    // mark resting orders that were fully matched as fulfilled
    if !fulfilled_ids.is_empty() {
        sqlx::query("UPDATE orders SET status='fulfilled' WHERE order_id = ANY($1)")
            .bind(&fulfilled_ids)
            .execute(pool)
            .await?;
    }

    // persist any remaining (unfilled) order into the database orderbook
    let mut appended_order_id = None;
    if let Some(order) = appends {
        appended_order_id = Some(sync_orderbook(pool, &order).await?);
    }

    Ok((balances[&user_id].clone(), appended_order_id))
}

pub async fn update_balance(
    user_id: i32,
    side: &str,
    qty: i32,
    price: i32,
    pool: &PgPool,
) -> Result<Balances, sqlx::Error> {
    // updates the balance of users as per each new Order in the Orderbook is added
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
