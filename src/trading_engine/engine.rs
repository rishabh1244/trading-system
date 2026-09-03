use crate::domain::common::Balances;
use crate::domain::order::{Order, PendingOrderRow};
use crate::domain::trades::TradeList;
use crate::matching_engine::orderbook::OrderBook;
use crate::trading_engine::orderbook::sync_orderbook;
use rust_decimal::Decimal;
use sqlx::{PgPool, Postgres, Transaction};
use std::collections::{HashMap, HashSet};

pub async fn settle_trades(
    user_id: i32,
    incoming: &Order,
    trade: TradeList,
    appends: Option<Order>,
    fulfilled_ids: Vec<i32>,
    tx: &mut Transaction<'_, Postgres>,
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
            "SELECT user_id, balance_btc, balance_inr, reserved_btc, reserved_inr \
             from balances where user_id=$1",
        )
        .bind(id)
        .fetch_one(&mut **tx)
        .await?;
        balances.insert(id, balance);
    }

    for trade in trade.trades.iter() {
        let qty = trade.qty;
        let value = trade.qty * trade.price;
        // buyer pays INR (out of his reserved inr) and receives BTC
        {
            let Some(buyer) = balances.get_mut(&trade.buyer_id) else {
                return Err(sqlx::Error::RowNotFound);
            };
            buyer.balance_btc += qty;
            buyer.reserved_inr -= value;
        }

        // seller gives BTC (out of his reserved btc) and receives INR
        {
            let Some(seller) = balances.get_mut(&trade.seller_id) else {
                return Err(sqlx::Error::RowNotFound);
            };
            seller.balance_inr += value;
            seller.reserved_btc -= qty;
        }
        // updates trades to datbase
        sqlx::query(
            "INSERT INTO trades (buyer_id , seller_id ,qty , price) VALUES ($1, $2, $3, $4)",
        )
        .bind(trade.buyer_id)
        .bind(trade.seller_id)
        .bind(trade.qty)
        .bind(trade.price)
        .execute(&mut **tx)
        .await?;
    }

    // release any reserve of the incoming order that was NOT consumed by a trade
    // and is NOT carried over into the resting (appended) leftover. this happens
    // when the order fills at a better price than its limit.
    let incoming_reserved = if incoming.side == "BUY" {
        Decimal::from(incoming.qty) * Decimal::from(incoming.price)
    } else {
        Decimal::from(incoming.qty)
    };
    let mut consumed = Decimal::ZERO;
    for t in trade.trades.iter() {
        if incoming.side == "BUY" {
            if t.buyer_id == incoming.user_id {
                consumed += t.qty * t.price;
            }
        } else if t.seller_id == incoming.user_id {
            consumed += t.qty;
        }
    }
    let mut leftover_reserved = Decimal::ZERO;
    if let Some(o) = &appends {
        if o.user_id == incoming.user_id {
            leftover_reserved = if incoming.side == "BUY" {
                Decimal::from(o.qty) * Decimal::from(o.price)
            } else {
                Decimal::from(o.qty)
            };
        }
    }
    let excess = incoming_reserved - consumed - leftover_reserved;
    if excess > Decimal::ZERO {
        let balance = balances.get_mut(&user_id).ok_or(sqlx::Error::RowNotFound)?;
        // refund the unused reservation back into the available balance
        if incoming.side == "BUY" {
            balance.reserved_inr -= excess;
            balance.balance_inr += excess;
        } else {
            balance.reserved_btc -= excess;
            balance.balance_btc += excess;
        }
    }

    for (id, balance) in balances.iter() {
        sqlx::query(
            "UPDATE balances SET balance_btc=$1, balance_inr=$2, reserved_btc=$3, reserved_inr=$4 \
             WHERE user_id=$5",
        )
        .bind(balance.balance_btc)
        .bind(balance.balance_inr)
        .bind(balance.reserved_btc)
        .bind(balance.reserved_inr)
        .bind(id)
        .execute(&mut **tx)
        .await?;
    }

    // mark resting orders that were fully matched as fulfilled
    if !fulfilled_ids.is_empty() {
        sqlx::query("UPDATE orders SET status='fulfilled' WHERE order_id = ANY($1)")
            .bind(&fulfilled_ids)
            .execute(&mut **tx)
            .await?;
    }

    // persist any remaining (unfilled) order into the database orderbook
    let mut appended_order_id = None;
    if let Some(order) = appends {
        appended_order_id = Some(sync_orderbook(tx, &order).await?);
    }

    Ok((balances[&user_id].clone(), appended_order_id))
}
pub async fn update_orderbook(orderbook: &mut OrderBook, pool: &PgPool) -> Result<(), sqlx::Error> {
    let rows = sqlx::query_as::<_, PendingOrderRow>(
        "SELECT order_id, user_id, side, qty::int4, price::int4
             FROM orders
             WHERE status = 'pending'
             ORDER BY order_id",
    )
    .fetch_all(pool)
    .await?;

    for row in rows {
        let order = Order {
            order_id: Some(row.order_id),
            user_id: row.user_id,
            side: row.side.to_uppercase(),
            qty: row.qty.into(),
            price: row.price.into(),
            status: "pending".to_string(),
        };
        orderbook.add_resting_order(order);
    }
    Ok(())
}
