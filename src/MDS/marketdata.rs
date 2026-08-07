use sqlx::PgPool;

use serde::{Deserialize, Serialize};
use std::cmp;
/*
 this service maintains
         best bid
         best ask
         last traded price
         recent trades
         order book depth
         candles
*/

pub fn service() {}
