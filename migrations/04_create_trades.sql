CREATE TABLE trades (
    trade_id SERIAL PRIMARY KEY,
    buyer_id INTEGER REFERENCES users(id),
    seller_id INTEGER REFERENCES users(id),
    qty NUMERIC NOT NULL,
    price NUMERIC NOT NULL,
    timestamp TIMESTAMP NOT NULL DEFAULT NOW()
);
