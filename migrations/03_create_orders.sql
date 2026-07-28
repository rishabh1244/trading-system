CREATE TABLE orders (
    order_id SERIAL PRIMARY KEY,
    user_id INTEGER REFERENCES users(id),
    side TEXT NOT NULL CHECK (side IN ('buy', 'sell')),
    qty NUMERIC NOT NULL,
    price NUMERIC NOT NULL,
    dateadded TIMESTAMP NOT NULL DEFAULT NOW(),
    status TEXT NOT NULL
);
