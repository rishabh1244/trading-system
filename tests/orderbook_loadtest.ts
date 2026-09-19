import http from 'k6/http';
import { check } from 'k6';

export const options = {
  stages: [
    { duration: '10s', target: 10 },
    { duration: '20s', target: 25 },
    { duration: '20s', target: 50 },
    { duration: '20s', target: 100 },
    { duration: '10s', target: 0 },
  ],

  thresholds: {
    http_req_failed: ['rate<0.01'],
    http_req_duration: ['p(95)<1800'],
  },
};

const BASE_URL = 'http://localhost:8080';

export function setup() {
  const loginRes = http.post(`${BASE_URL}/api/login`, JSON.stringify({
    username: 'rishabh',
    password: 'rishabh123',
  }), {
    headers: { 'Content-Type': 'application/json' },
  });

  check(loginRes, {
    'login succeeded': (r) => r.status === 200,
  });

  const body = JSON.parse(loginRes.body as string);
  return { token: body.token };
}

export default function(data: { token: string }) {
 

  const headers = {
    Authorization: `Bearer ${data.token}`,
    'Content-Type': 'application/json',
  };

  const side = Math.random() < 0.5 ? 'BUY' : 'SELL';
  const price = Math.floor(Math.random() * 50000) + 95000;
  const qty = Math.floor(Math.random() * 10) + 1;

  const orderRes = http.post(`${BASE_URL}/api/order`, JSON.stringify({
    side,
    qty,
    price,
  }), { headers });

  check(orderRes, {
    'order status is 200': (r) => r.status === 200,
  });
}
