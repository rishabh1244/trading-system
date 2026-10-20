import http from 'k6/http';
import { check, sleep } from 'k6';

const APP_URL = __ENV.APP_URL || 'http://127.0.0.1:8080';
const PROMETHEUS_URL = __ENV.PROMETHEUS_URL || 'http://127.0.0.1:9090';

function targetIsUp(body: string): boolean {
  try {
    return JSON.parse(body).data.result.some(
      (series: {
        metric: { job?: string };
        value: [string, string];
      }) => series.metric.job === 'trading_engine' && series.value[1] === '1',
    );
  } catch (_) {
    return false;
  }
}

export const options = {
  scenarios: {
    prometheus_check: {
      executor: 'constant-vus',
      vus: 2,
      duration: '30s',
    },
  },
  thresholds: {
    http_req_failed: ['rate<0.01'],
    checks: ['rate>0.99'],
  },
};

export default function () {
  const metrics = http.get(`${APP_URL}/metrics`);
  check(metrics, {
    'application metrics endpoint is reachable': (res) => res.status === 200,
    'application returns Prometheus format': (res) =>
      res.status === 200 &&
      typeof res.body === 'string' &&
      res.body.includes('trading_engine_stage_count'),
  });

  const query = encodeURIComponent('up{job="trading_engine"}');
  const prometheus = http.get(
    `${PROMETHEUS_URL}/api/v1/query?query=${query}`,
  );
  check(prometheus, {
    'Prometheus query succeeds': (res) => res.status === 200,
    'trading engine target is up': (res) =>
      res.status === 200 &&
      typeof res.body === 'string' &&
      targetIsUp(res.body),
  });

  sleep(1);
}
