// k6 load test for chat completions API
// Run with: k6 run -e API_KEY=your-key tests/load/chat.js

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const chatLatency = new Trend('chat_latency');
const tokensProcessed = new Counter('tokens_processed');

// Test configuration
export const options = {
  scenarios: {
    // Smoke test
    smoke: {
      executor: 'constant-vus',
      vus: 1,
      duration: '10s',
      startTime: '0s',
      tags: { scenario: 'smoke' },
    },
    // Sustained load
    sustained: {
      executor: 'constant-arrival-rate',
      rate: 10,          // 10 requests per second
      timeUnit: '1s',
      duration: '2m',
      preAllocatedVUs: 20,
      maxVUs: 50,
      startTime: '15s',
      tags: { scenario: 'sustained' },
    },
    // Spike test
    spike: {
      executor: 'ramping-arrival-rate',
      startRate: 5,
      timeUnit: '1s',
      stages: [
        { duration: '10s', target: 50 },  // Spike to 50 rps
        { duration: '30s', target: 50 },  // Hold
        { duration: '10s', target: 5 },   // Return to normal
      ],
      preAllocatedVUs: 100,
      maxVUs: 200,
      startTime: '2m30s',
      tags: { scenario: 'spike' },
    },
  },
  thresholds: {
    http_req_duration: ['p(95)<5000', 'p(99)<10000'], // LLM calls can be slow
    http_req_failed: ['rate<0.05'],
    errors: ['rate<0.05'],
    chat_latency: ['p(95)<5000'],
  },
};

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';
const API_KEY = __ENV.API_KEY || 'test-api-key';

const MODELS = ['gpt-4', 'gpt-3.5-turbo', 'claude-3-sonnet'];
const PROMPTS = [
  'Hello, how are you?',
  'What is the capital of France?',
  'Explain quantum computing in simple terms.',
  'Write a haiku about programming.',
  'What are the benefits of Rust?',
];

export default function () {
  const model = MODELS[Math.floor(Math.random() * MODELS.length)];
  const prompt = PROMPTS[Math.floor(Math.random() * PROMPTS.length)];

  const payload = JSON.stringify({
    model: model,
    messages: [
      { role: 'system', content: 'You are a helpful assistant.' },
      { role: 'user', content: prompt },
    ],
    max_tokens: 100,
    temperature: 0.7,
  });

  const params = {
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${API_KEY}`,
    },
    timeout: '30s',
  };

  const res = http.post(`${BASE_URL}/v1/chat/completions`, payload, params);
  chatLatency.add(res.timings.duration);

  const success = check(res, {
    'chat status is 200': (r) => r.status === 200,
    'chat response has choices': (r) => {
      try {
        const body = JSON.parse(r.body);
        return body.choices && body.choices.length > 0;
      } catch {
        return false;
      }
    },
    'chat response has content': (r) => {
      try {
        const body = JSON.parse(r.body);
        return body.choices[0].message?.content?.length > 0;
      } catch {
        return false;
      }
    },
  });

  errorRate.add(!success);

  // Track token usage if available
  try {
    const body = JSON.parse(res.body);
    if (body.usage) {
      tokensProcessed.add(body.usage.total_tokens || 0);
    }
  } catch {
    // Ignore parsing errors
  }

  sleep(0.5); // Delay between requests
}

export function handleSummary(data) {
  return {
    'stdout': textSummary(data),
    'tests/load/results/chat-summary.json': JSON.stringify(data),
  };
}

function textSummary(data) {
  let summary = `\n=== Chat Completions Load Test Summary ===\n\n`;

  summary += `Duration: ${data.state.testRunDurationMs}ms\n`;
  summary += `VUs Max: ${data.metrics.vus_max?.values?.max || 0}\n`;
  summary += `Iterations: ${data.metrics.iterations?.values?.count || 0}\n\n`;

  summary += `HTTP Requests:\n`;
  summary += `  Total: ${data.metrics.http_reqs?.values?.count || 0}\n`;
  summary += `  Failed: ${data.metrics.http_req_failed?.values?.passes || 0}\n`;
  summary += `  Avg Duration: ${(data.metrics.http_req_duration?.values?.avg || 0).toFixed(2)}ms\n`;
  summary += `  P95 Duration: ${(data.metrics.http_req_duration?.values?.['p(95)'] || 0).toFixed(2)}ms\n`;
  summary += `  P99 Duration: ${(data.metrics.http_req_duration?.values?.['p(99)'] || 0).toFixed(2)}ms\n\n`;

  summary += `Chat Metrics:\n`;
  summary += `  Avg Latency: ${(data.metrics.chat_latency?.values?.avg || 0).toFixed(2)}ms\n`;
  summary += `  P95 Latency: ${(data.metrics.chat_latency?.values?.['p(95)'] || 0).toFixed(2)}ms\n`;
  summary += `  Tokens Processed: ${data.metrics.tokens_processed?.values?.count || 0}\n`;
  summary += `  Error Rate: ${((data.metrics.errors?.values?.rate || 0) * 100).toFixed(2)}%\n`;

  return summary;
}
