// k6 load test for health endpoints
// Run with: k6 run tests/load/health.js

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const healthLatency = new Trend('health_latency');
const readyLatency = new Trend('ready_latency');
const liveLatency = new Trend('live_latency');

// Test configuration
export const options = {
  scenarios: {
    // Smoke test - verify basic functionality
    smoke: {
      executor: 'constant-vus',
      vus: 1,
      duration: '10s',
      startTime: '0s',
      tags: { scenario: 'smoke' },
    },
    // Load test - normal expected load
    load: {
      executor: 'ramping-vus',
      startVUs: 0,
      stages: [
        { duration: '30s', target: 50 },  // Ramp up to 50 users
        { duration: '1m', target: 50 },   // Stay at 50 users
        { duration: '30s', target: 0 },   // Ramp down
      ],
      startTime: '15s',
      tags: { scenario: 'load' },
    },
    // Stress test - beyond normal capacity
    stress: {
      executor: 'ramping-vus',
      startVUs: 0,
      stages: [
        { duration: '30s', target: 100 },
        { duration: '1m', target: 200 },
        { duration: '30s', target: 0 },
      ],
      startTime: '2m30s',
      tags: { scenario: 'stress' },
    },
  },
  thresholds: {
    http_req_duration: ['p(95)<200', 'p(99)<500'],
    http_req_failed: ['rate<0.01'],
    errors: ['rate<0.01'],
  },
};

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';

export default function () {
  // Test /health endpoint
  const healthRes = http.get(`${BASE_URL}/health`);
  healthLatency.add(healthRes.timings.duration);

  const healthCheck = check(healthRes, {
    'health status is 200': (r) => r.status === 200,
    'health response has status': (r) => {
      try {
        const body = JSON.parse(r.body);
        return body.status === 'healthy';
      } catch {
        return false;
      }
    },
    'health response time < 100ms': (r) => r.timings.duration < 100,
  });
  errorRate.add(!healthCheck);

  // Test /ready endpoint
  const readyRes = http.get(`${BASE_URL}/ready`);
  readyLatency.add(readyRes.timings.duration);

  const readyCheck = check(readyRes, {
    'ready status is 200': (r) => r.status === 200,
    'ready response time < 200ms': (r) => r.timings.duration < 200,
  });
  errorRate.add(!readyCheck);

  // Test /live endpoint
  const liveRes = http.get(`${BASE_URL}/live`);
  liveLatency.add(liveRes.timings.duration);

  const liveCheck = check(liveRes, {
    'live status is 200': (r) => r.status === 200,
    'live response time < 50ms': (r) => r.timings.duration < 50,
  });
  errorRate.add(!liveCheck);

  sleep(0.1); // Small delay between iterations
}

export function handleSummary(data) {
  return {
    'stdout': textSummary(data, { indent: ' ', enableColors: true }),
    'tests/load/results/health-summary.json': JSON.stringify(data),
  };
}

function textSummary(data, opts) {
  const indent = opts.indent || '';
  let summary = `\n${indent}=== Load Test Summary ===\n\n`;

  summary += `${indent}Duration: ${data.state.testRunDurationMs}ms\n`;
  summary += `${indent}VUs Max: ${data.metrics.vus_max?.values?.max || 0}\n`;
  summary += `${indent}Iterations: ${data.metrics.iterations?.values?.count || 0}\n\n`;

  summary += `${indent}HTTP Requests:\n`;
  summary += `${indent}  Total: ${data.metrics.http_reqs?.values?.count || 0}\n`;
  summary += `${indent}  Failed: ${data.metrics.http_req_failed?.values?.passes || 0}\n`;
  summary += `${indent}  Avg Duration: ${(data.metrics.http_req_duration?.values?.avg || 0).toFixed(2)}ms\n`;
  summary += `${indent}  P95 Duration: ${(data.metrics.http_req_duration?.values?.['p(95)'] || 0).toFixed(2)}ms\n`;
  summary += `${indent}  P99 Duration: ${(data.metrics.http_req_duration?.values?.['p(99)'] || 0).toFixed(2)}ms\n\n`;

  summary += `${indent}Custom Metrics:\n`;
  summary += `${indent}  Health Avg: ${(data.metrics.health_latency?.values?.avg || 0).toFixed(2)}ms\n`;
  summary += `${indent}  Ready Avg: ${(data.metrics.ready_latency?.values?.avg || 0).toFixed(2)}ms\n`;
  summary += `${indent}  Live Avg: ${(data.metrics.live_latency?.values?.avg || 0).toFixed(2)}ms\n`;
  summary += `${indent}  Error Rate: ${((data.metrics.errors?.values?.rate || 0) * 100).toFixed(2)}%\n`;

  return summary;
}
