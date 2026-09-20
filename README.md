


# Centralized Trading System

<img width="6365" height="4412" alt="Untitled-2026-01-30-0959 excalidraw(1)" src="https://github.com/user-attachments/assets/6e9c23c1-d833-416e-8689-1804d9cb9c67" />

A production-inspired centralized cryptocurrency trading system built from scratch in Rust.

The project focuses on order matching, concurrency, transactional consistency, real-time market data, and backend performance.

## Tech Stack

- Rust
- PostgreSQL
- WebSockets
- Grafana k6

## Architecture

The system is divided into several components:

- **API Gateway & Auth** — entry point for client requests, authentication, rate limiting and WebSocket connections.
- **Order Management Service** — validates incoming orders, verifies balances, trading pairs, quantities and other constraints.
- **Matching Engine** — maintains the order book and matches BUY/SELL orders using price-time priority.
- **Trading Engine** — handles trade execution, settlement, balance updates and persistence.
- **Market Data Service** — maintains market statistics, recent trades and real-time market information.
- **PostgreSQL** — persistent storage for users, balances, orders and trades.

## Order Flow

A simplified order flow looks like:

```
Client
  ↓
API Gateway
  ↓
Order Management
  ↓
Balance Reservation
  ↓
Matching Engine
  ↓
Trade Execution
  ↓
Settlement
  ↓
PostgreSQL
  ↓
Market Data / WebSocket Updates
````

## Order Book

The order book is maintained in memory for fast matching.

```text
Bids                         Asks

100.50 → [Order, Order]      100.60 → [Order]
100.40 → [Order]             100.70 → [Order, Order]
100.30 → [Order]             100.80 → [Order]
```

Orders at the same price are processed in FIFO order.

The order book uses Rust's `BTreeMap` for price-level organization and `VecDeque` for maintaining FIFO ordering within each price level.

## Matching Engine

The matching engine supports:

* BUY and SELL orders
* Price-time priority
* Partial fills
* Full fills
* Limit orders
* Order cancellation
* Maintaining bid and ask price levels

The matching engine operates on the in-memory order book, while trade and account state are persisted to PostgreSQL.

## Concurrency & Consistency

One of the more challenging parts of the project was keeping balances, orders and trades consistent when multiple orders are processed concurrently.

Simply checking a user's balance before placing an order is not sufficient when multiple transactions can observe and modify the same state at the same time.

The system therefore uses transactional balance reservation and settlement to ensure that state changes across balances, orders and trades remain consistent.

A major part of development involved identifying concurrency issues and defining the correct boundaries between:

```text
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

This was one of the main reasons for introducing database transactions and synchronization around shared in-memory state.

## Testing

The project includes:

* Unit tests
* Integration tests
* End-to-end API tests
* Concurrent order scenarios
* Database transaction tests
* Load testing with k6

## Load Testing

Load testing is performed using **Grafana k6**.

One of the current k6 runs processed:

```text
Total requests/checks: 20,355
Successful:             20,294
Failed:                     61
Failure rate:            0.29%

p95 latency:            ~989 ms
```

### Current Status

The remaining failed requests are currently being investigated.

The current load test exposes a deadlock under concurrent load, which is being worked on.

The failure is intentionally documented here rather than hidden, since identifying and resolving concurrency issues is an important part of the development process.

## Performance Instrumentation

The application includes latency instrumentation for different parts of the order processing pipeline.

The goal is to measure individual stages rather than treating the entire HTTP request as a black box.

Current instrumentation includes operations such as:

```text
Database transaction
Balance reservation
Order book access
Order matching
Trade execution
Settlement
```

This makes it possible to identify where latency is being introduced and distinguish between database, synchronization and application-level bottlenecks.

## API

### Authentication

```http
POST /login
POST /register
```

### Orders

```http
POST   /order
DELETE /order/:id
```

### Account & Market Data

```http
GET /balances
GET /market-data
GET /recent-trades
```

### WebSockets

WebSocket connections provide real-time updates for:

* Price updates
* Order status updates
* Trade updates

## Database

PostgreSQL is used for persistent application state.

The database stores:

```text
Users
 └── user_id
 └── username
 └── password

Balances
 └── user_id
 └── balance_btc
 └── balance_inr

Orders
 └── order_id
 └── user_id
 └── side
 └── quantity
 └── price
 └── status

Trades
 └── trade_id
 └── buyer_id
 └── seller_id
 └── quantity
 └── price
 └── timestamp
```

Financial values are represented using `rust_decimal` rather than floating-point arithmetic.

## Running Locally

### Requirements

* Rust
* PostgreSQL
* Docker (optional)
* SQLx CLI
* k6 (for load testing)

### Clone the repository

```bash
git clone https://github.com/rishabh1244/trading-system.git
cd trading-system
```

### Configure environment variables

Create a `.env` file containing the required database and application configuration.

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

## Project Structure

```text
src/
├── ...
tests/
├── ...
migrations/
├── ...
```

The project structure is organized around the API layer, order management, matching engine, trading/settlement logic, market data and persistence.

## What I Learned

This project started as an attempt to build a trading system, but the most interesting problems ended up being around systems correctness.

Some of the main things I worked through:

* Concurrent state modification
* Database transaction boundaries
* Atomic balance reservation
* Order-book synchronization
* Price-time priority matching
* Financial arithmetic using `Decimal`
* Failure handling
* End-to-end testing
* Load testing
* Latency instrumentation
* Identifying synchronization bottlenecks

The project also gave me a much better understanding of the difference between simply building an API and engineering a stateful system where multiple components have to remain consistent.

## Future Work

* [ ] Resolve the remaining concurrency/deadlock issue
* [ ] Improve matching-engine benchmarks
* [ ] Reduce order-path latency
* [ ] Expand concurrent stress testing
* [ ] Improve failure recovery
* [ ] Add more detailed performance benchmarks


