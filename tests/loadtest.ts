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
        http_req_duration: ['p(95)<500'],
    },
};

const BASE_URL = 'http://localhost:8080';

const TOKEN = 'eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJ1c2VybmFtZSI6InJpc2hhYmgiLCJleHAiOjE3ODk3MTAxNTcsImlkIjoxfQ.5HGypQ9wzUzI5AOKzvReBiDzOy9zCNsuBcayeKnak6g';

export default function () {
    const res = http.get(`${BASE_URL}/api/balance`, {
        headers: {
            Authorization: `Bearer ${TOKEN}`,
        },
    });

    check(res, {
        'status is 200': (r) => r.status === 200,
    });
}

