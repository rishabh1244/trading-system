#!/usr/bin/env python3
"""Minimal smoke test: log in as one seller and one buyer, send a single
SELL and a single BUY, then exit. No k6, no load, no assertions on state."""

import json
import os
import sys
import urllib.error
import urllib.request

BASE_URL = os.environ.get(
    "SMOKE_URL", os.environ.get("LOADTEST_URL", "http://localhost:8080")
)
HERE = os.path.dirname(os.path.abspath(__file__))
USERS_PATH = os.environ.get("SMOKE_USERS", os.path.join(HERE, "loadtest_users.json"))

QTY = 1
PRICE = 100


def load_creds():
    try:
        with open(USERS_PATH) as f:
            users = json.load(f)
    except FileNotFoundError:
        users = [
            {"username": "bench_0", "password": "pass_0", "role": "seller"},
            {"username": "bench_50", "password": "pass_50", "role": "buyer"},
        ]
    sellers = [u for u in users if u.get("role") == "seller"]
    buyers = [u for u in users if u.get("role") == "buyer"]
    if not sellers or not buyers:
        sys.exit(f"{USERS_PATH} must contain at least one seller and one buyer")
    return sellers[0], buyers[0]


def request(method, path, body=None, token=None):
    headers = {"Content-Type": "application/json"}
    if token:
        headers["Authorization"] = f"Bearer {token}"
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(
        f"{BASE_URL}{path}", data=data, headers=headers, method=method
    )
    try:
        with urllib.request.urlopen(req) as resp:
            return resp.status, json.loads(resp.read() or b"null")
    except urllib.error.HTTPError as e:
        return e.code, e.read().decode(errors="replace")


def login(user):
    status, body = request(
        "POST",
        "/api/login",
        {"username": user["username"], "password": user["password"]},
    )
    if status != 200:
        sys.exit(f"login failed for {user['username']}: {status} {body}")
    print(f"logged in {user['username']} ({user['role']})")
    return body["token"]


def main():
    seller, buyer = load_creds()
    seller_token = login(seller)
    buyer_token = login(buyer)

    sell_status, sell_body = request(
        "POST",
        "/api/order",
        {"side": "SELL", "qty": QTY, "price": PRICE},
        seller_token,
    )
    print(f"SELL {QTY}@{PRICE} -> {sell_status} {sell_body}")

    buy_status, buy_body = request(
        "POST",
        "/api/order",
        {"side": "BUY", "qty": QTY, "price": PRICE},
        buyer_token,
    )
    print(f"BUY  {QTY}@{PRICE} -> {buy_status} {buy_body}")

    if sell_status != 200 or buy_status != 200:
        sys.exit(1)
    print("ok: one sell + one buy accepted")


if __name__ == "__main__":
    main()
