

<h1 align="center">
  Centralized Trading System
</h1>

<img width="6365" height="4412" alt="Untitled-2026-01-30-0959 excalidraw(1)" src="https://github.com/user-attachments/assets/6e9c23c1-d833-416e-8689-1804d9cb9c67" />

A centralized cryptocurrency trading system built in Rust. Handles order matching, balance management, trade settlement, and real-time market data over WebSockets.

## Tech Stack

- Rust
- PostgreSQL
- WebSockets
- Grafana k6

## Architecture

```
API Gateway (actix-web :8080)
  ├── JWT Auth Middleware
  ├── Order Management Service (balance reservation, input validation)
  │     └── Matching Engine (in-memory order book, price-time priority)
  │           └── Trading Engine (settlement, DB persistence)
  ├── Market Data Service (last price, WebSocket broadcast :7878)
  └── Metrics (per-stage latency tracking)
```

PostgreSQL stores users, balances, orders, and trades. The order book lives in memory for fast matching. Only resting (unmatched) orders are persisted.

## Order Flow

```
Client
  ↓
API Gateway (JWT auth)
  ↓
Order Management (validate input, begin DB transaction)
  ↓
Balance Reservation (atomic UPDATE: balance → reserved)
  ↓
Matching Engine (price-time priority, partial/full fills)
  ↓
Trade Settlement (credit buyer BTC, seller INR, insert trades)
  ↓
Commit Transaction
  ↓
WebSocket Broadcast (last trade price to all clients)
```

## Order Book

The order book is maintained in memory for fast matching.

```text
Bids                         Asks

100.50 → [Order1, Order2]      100.60 → [Order5]
100.40 → [Order3]             100.70 → [Order6, Order7]
100.30 → [Order4]             100.80 → [Order8]
```

Orders at the same price are processed in FIFO order.

The order book uses `BTreeMap` for price-level organization and `VecDeque` for FIFO ordering within each price level.

## Matching Engine

The matching engine supports:

- BUY and SELL orders
- Price-time priority
- Partial fills
- Full fills
- Limit orders
- Maintaining bid and ask price levels

The matching engine operates on the in-memory order book, while trade and account state are persisted to PostgreSQL.

## Concurrency & Consistency

Keeping balances, orders and trades consistent when multiple orders come in at the same time is the hard part.

Simply checking a user's balance before placing an order is not enough when multiple transactions can read and modify the same state concurrently.

The system uses transactional balance reservation and settlement so that changes across balances, orders and trades stay consistent.

The main concurrency boundaries are:

```
Balance Reservation
        ↓
Order Matching
        ↓
Trade Execution
        ↓
Settlement
        ↓
Persistence
```

The in-memory order book is behind `Arc<Mutex<OrderBook>>` and market data behind `Arc<Mutex<MarketData>>`. Balance reservation and settlement happen inside a PostgreSQL transaction for atomicity.

## Testing

- Unit tests (order book matching logic)
- Integration tests (full flow with real DB)
- End-to-end API tests (HTTP requests through the full pipeline)
- Concurrent order scenarios
- Database transaction tests
- Load testing with k6

## Load Testing

Load testing is done with **Grafana k6**.

One of the k6 runs so far:

```text
Total requests/checks: 20,355
    http_req_duration
    ✓ 'p(95)<1800' p(95)=251.84ms

    http_req_failed
    ✓ 'rate<0.01' rate=0.07%


```

The remaining failed are caused due to deadlock under concurrent load, which is being worked on. (for more info see issues/deadlock_faliure.txt) 

## Performance Instrumentation

The application tracks latency for different stages of the order processing pipeline instead of just measuring the whole HTTP request.

Current instrumentation covers:

```text
Database transaction
Balance reservation
Order book access
Order matching
Trade execution
Settlement
```

This helps identify where latency is coming from — database, synchronization, or application code.

## API

All endpoints are prefixed with `/api/`.

### Authentication

```http
POST /api/register
POST /api/login
```

### Orders

```http
POST /api/order
```

### Account

```http
GET  /api/balance
GET  /api/my-orders
GET  /api/orderbook
```

### Metrics

```http
GET /metrics
```

### WebSockets

WebSocket server runs on `ws://127.0.0.1:7878` and broadcasts:

- Last trade price
- Trade updates

## Database

PostgreSQL is used for persistent application state.

The database stores:

```text
Users
 └── id (SERIAL PRIMARY KEY)
 └── username (TEXT UNIQUE)
 └── password_hash (TEXT)
 └── created_at (TIMESTAMP)

Balances
 └── user_id (INTEGER REFERENCES users)
 └── balance_btc (NUMERIC)
 └── balance_inr (NUMERIC)
 └── reserved_btc (NUMERIC)
 └── reserved_inr (NUMERIC)

Orders
 └── order_id (SERIAL PRIMARY KEY)
 └── user_id (INTEGER REFERENCES users)
 └── side (TEXT: 'buy' or 'sell')
 └── qty (NUMERIC)
 └── price (NUMERIC)
 └── dateadded (TIMESTAMP)
 └── status (TEXT)

Trades
 └── trade_id (SERIAL PRIMARY KEY)
 └── buyer_id (INTEGER REFERENCES users)
 └── seller_id (INTEGER REFERENCES users)
 └── qty (NUMERIC)
 └── price (NUMERIC)
 └── timestamp (TIMESTAMP)
```

All financial values use `rust_decimal` instead of floating-point.

## Running Locally

### Requirements

- Rust
- PostgreSQL
- SQLx CLI
- k6 (for load testing)

### Clone the repository

```bash
git clone https://github.com/rishabh1244/trading-system.git
cd trading-system
```

### Configure environment variables

Create a `.env` file with the required database and application configuration.

```bash
cp .env.example .env
```

### Run database migrations

```bash
sqlx migrate run
```

### Start the application

```bash
cargo run
```

### Run tests

```bash
cargo test
```
### Run k6 load tests

#### Generate seed users for test
```bash
python3 ./tests/seed_users.py
```
#### Run the test with seed credentials
```bash
k6 run ./tests/orderbook_loadtest.ts
```

## Project Structure

```text
src/
├── main.rs
├── lib.rs
├── metrics.rs
├── api_gateway/      (HTTP server, route registration, DB pool, metrics endpoint)
├── auth/             (register, login handlers)
├── middleware/        (JWT Bearer token validation)
├── domain/           (data types: Order, Trade, User, Balances, MarketData)
├── OMS/              (order management, balance reservation)
├── matching_engine/  (in-memory order book, price-time priority matching)
├── trading_engine/   (settlement, DB persistence)
└── MDS/              (market data, WebSocket broadcast)

tests/
├── orderbook_test.rs
├── e2e_orderbook_test.rs
├── metrics_test.rs
├── api/
│   ├── auth_test.rs
│   └── e2e_trade_test.rs
├── loadtest.ts
├── orderbook_loadtest.ts
└── seed_users.py

migrations/
├── 01_create_user.sql
├── 02_create_balances.sql
├── 03_create_orders.sql
└── 04_create_trades.sql
```

## What I Learned

This project started as an attempt to build a trading system, but the interesting problems ended up being around systems correctness.

Some of the main things I worked through:

- Concurrent state modification
- Database transaction boundaries
- Atomic balance reservation
- Order-book synchronization
- Price-time priority matching
- Financial arithmetic using `Decimal`
- Failure handling
- End-to-end testing
- Load testing
- Latency instrumentation
- Identifying synchronization bottlenecks

The project gave me a better understanding of the difference between building an API and engineering a stateful system where multiple components have to stay consistent.

## Future Work

- [ ] Resolve the remaining concurrency/deadlock issue
- [ ] Improve matching-engine benchmarks
- [ ] Reduce order-path latency
- [ ] Expand concurrent stress testing
- [ ] Improve failure recovery
- [ ] Add more detailed performance benchmarks
