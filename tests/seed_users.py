#!/usr/bin/env python3
"""Seed 100 users with balances. Run once before the loadtest."""

import json
import os
import subprocess
import sys
import urllib.request
import urllib.error

BASE_URL = os.environ.get("LOADTEST_URL", "http://localhost:8080")
DATABASE_URL = os.environ.get("DATABASE_URL", "postgres://localhost:5432/trading_engine")
NUM_USERS = 100
NUM_SELLERS = 50
SELLER_BTC = 1000
BUYER_INR = 10_000_000
OUT_PATH = os.path.join(os.path.dirname(__file__), "loadtest_users.json")

users = []
for i in range(NUM_USERS):
    username = f"bench_{i}"
    password = f"pass_{i}"
    role = "seller" if i < NUM_SELLERS else "buyer"

    try:
        req = urllib.request.Request(
            f"{BASE_URL}/api/register",
            data=json.dumps({"username": username, "password": password}).encode(),
            headers={"Content-Type": "application/json"},
        )
        with urllib.request.urlopen(req) as resp:
            if resp.status == 200:
                users.append({"username": username, "password": password, "role": role})
                print(f"  registered {username} ({role})")
    except urllib.error.HTTPError:
        users.append({"username": username, "password": password, "role": role})
        print(f"  {username} already exists, saved")

# Set balances via psql
sql = ""
for u in users:
    sql += f"DELETE FROM balances WHERE user_id = (SELECT id FROM users WHERE username = '{u['username']}'); "
    if u["role"] == "seller":
        sql += f"INSERT INTO balances (user_id, balance_btc, balance_inr, reserved_btc, reserved_inr) VALUES ((SELECT id FROM users WHERE username = '{u['username']}'), {SELLER_BTC}, 0, 0, 0); "
    else:
        sql += f"INSERT INTO balances (user_id, balance_btc, balance_inr, reserved_btc, reserved_inr) VALUES ((SELECT id FROM users WHERE username = '{u['username']}'), 0, {BUYER_INR}, 0, 0); "

print("\nSetting balances via psql...")
result = subprocess.run(["psql", DATABASE_URL, "-c", sql], capture_output=True, text=True)
if result.returncode != 0:
    print(f"psql error: {result.stderr}", file=sys.stderr)
    sys.exit(1)
print("Balances set.")

with open(OUT_PATH, "w") as f:
    json.dump(users, f, indent=2)

print(f"\nWrote {len(users)} users to {OUT_PATH}")
print("Run: k6 run tests/orderbook_loadtest.ts")
