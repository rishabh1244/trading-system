CREATE TABLE balances (
    user_id INTEGER REFERENCES users(id),
 
    balance_btc NUMERIC NOT NULL DEFAULT 0,
    balance_inr NUMERIC NOT NULL DEFAULT 0,
 
    reserved_btc NUMERIC NOT NULL DEFAULT 0,
    reserved_btc NUMERIC NOT NULL DEFAULT 0
);
