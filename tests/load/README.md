# Load Tests

Load tests for the LLM Gateway API using [k6](https://k6.io/).

## Prerequisites

- [k6](https://k6.io/docs/getting-started/installation/) installed locally, OR
- Docker for running k6 in a container

## Test Files

| File | Description |
|------|-------------|
| `health.js` | Tests health endpoints (/health, /ready, /live) |
| `chat.js` | Tests chat completions API with various prompts |

## Running Tests

### With k6 installed locally

```bash
# Health endpoints test
k6 run tests/load/health.js

# Chat completions test (requires API key and running mock server)
k6 run -e API_KEY=your-api-key tests/load/chat.js

# With custom base URL
k6 run -e BASE_URL=http://localhost:3000 tests/load/health.js
```

### With Docker

```bash
# Health endpoints test
docker run --rm -i --network=host grafana/k6 run - < tests/load/health.js

# With environment variables
docker run --rm -i --network=host \
  -e BASE_URL=http://localhost:8080 \
  -e API_KEY=your-api-key \
  grafana/k6 run - < tests/load/chat.js
```

### With Docker Compose (for full stack testing)

```bash
# Start the application with mock services
bin/up.bat full

# In another terminal, run load tests
docker run --rm -i --network=pmp-llm-gateway-api_default \
  -e BASE_URL=http://app:8080 \
  grafana/k6 run - < tests/load/health.js
```

## Test Scenarios

### Health Tests (`health.js`)

1. **Smoke** (10s): Single VU verifying basic functionality
2. **Load** (2m): Ramp to 50 VUs, sustained load
3. **Stress** (2m): Ramp to 200 VUs, beyond normal capacity

### Chat Tests (`chat.js`)

1. **Smoke** (10s): Single VU basic verification
2. **Sustained** (2m): 10 requests/second constant rate
3. **Spike** (50s): Spike to 50 rps then return to normal

## Thresholds

### Health Endpoints
- P95 latency < 200ms
- P99 latency < 500ms
- Error rate < 1%

### Chat Completions
- P95 latency < 5000ms
- P99 latency < 10000ms
- Error rate < 5%

## Results

Test results are written to `tests/load/results/`:
- `health-summary.json`
- `chat-summary.json`

## Metrics Collected

### Standard Metrics
- `http_reqs`: Total HTTP requests
- `http_req_duration`: Request duration
- `http_req_failed`: Failed request rate

### Custom Metrics
- `health_latency`: /health endpoint latency
- `ready_latency`: /ready endpoint latency
- `live_latency`: /live endpoint latency
- `chat_latency`: Chat completion latency
- `tokens_processed`: Total tokens processed
- `errors`: Custom error rate

## Tips

1. Start with smoke tests to verify setup
2. Monitor system resources during stress tests
3. Use `--out influxdb=...` to send metrics to InfluxDB for visualization
4. Use `--out json=results.json` for detailed results analysis
