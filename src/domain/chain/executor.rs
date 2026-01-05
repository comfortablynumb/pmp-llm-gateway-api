//! Chain executor - Handles chain execution with retry, fallback, and circuit breaker

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use tokio::sync::RwLock;
use tokio::time::timeout;

use super::{ChainStep, FallbackBehavior, ModelChain};
use crate::domain::{DomainError, LlmProvider, LlmRequest, LlmResponse, ModelId};

/// Configuration for the chain executor
#[derive(Debug, Clone)]
pub struct ChainExecutorConfig {
    /// Circuit breaker failure threshold before opening
    pub circuit_breaker_threshold: u32,
    /// Duration to keep circuit breaker open before half-open
    pub circuit_breaker_reset_ms: u64,
    /// Whether to collect metrics
    pub collect_metrics: bool,
}

impl Default for ChainExecutorConfig {
    fn default() -> Self {
        Self {
            circuit_breaker_threshold: 5,
            circuit_breaker_reset_ms: 30000, // 30 seconds
            collect_metrics: true,
        }
    }
}

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

/// Circuit breaker for a single model
#[derive(Debug)]
struct CircuitBreaker {
    failure_count: AtomicU32,
    last_failure_time: AtomicU64,
    threshold: u32,
    reset_duration_ms: u64,
}

impl CircuitBreaker {
    fn new(threshold: u32, reset_duration_ms: u64) -> Self {
        Self {
            failure_count: AtomicU32::new(0),
            last_failure_time: AtomicU64::new(0),
            threshold,
            reset_duration_ms,
        }
    }

    fn state(&self) -> CircuitState {
        let failures = self.failure_count.load(Ordering::Relaxed);

        if failures < self.threshold {
            return CircuitState::Closed;
        }

        let last_failure = self.last_failure_time.load(Ordering::Relaxed);
        let now = current_time_ms();

        if now - last_failure >= self.reset_duration_ms {
            CircuitState::HalfOpen
        } else {
            CircuitState::Open
        }
    }

    fn record_success(&self) {
        self.failure_count.store(0, Ordering::Relaxed);
    }

    fn record_failure(&self) {
        self.failure_count.fetch_add(1, Ordering::Relaxed);
        self.last_failure_time
            .store(current_time_ms(), Ordering::Relaxed);
    }

    fn is_available(&self) -> bool {
        matches!(self.state(), CircuitState::Closed | CircuitState::HalfOpen)
    }
}

fn current_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Metrics for chain execution
#[derive(Debug, Default, Clone)]
pub struct ChainMetrics {
    /// Total requests processed
    pub total_requests: u64,
    /// Successful requests
    pub successful_requests: u64,
    /// Failed requests
    pub failed_requests: u64,
    /// Total latency in milliseconds
    pub total_latency_ms: u64,
    /// Per-step metrics
    pub step_metrics: HashMap<String, StepMetrics>,
}

/// Metrics for a single step
#[derive(Debug, Default, Clone)]
pub struct StepMetrics {
    /// Times this step was attempted
    pub attempts: u64,
    /// Successful attempts
    pub successes: u64,
    /// Failed attempts
    pub failures: u64,
    /// Total retries
    pub retries: u64,
    /// Times circuit breaker was open
    pub circuit_breaker_trips: u64,
    /// Average latency in milliseconds
    pub avg_latency_ms: f64,
}

/// Result of executing a single step
#[derive(Debug)]
pub struct StepResult {
    /// Model ID that was used
    pub model_id: ModelId,
    /// Whether the step succeeded
    pub success: bool,
    /// Number of attempts made
    pub attempts: u32,
    /// Latency in milliseconds
    pub latency_ms: u64,
    /// Error message if failed
    pub error: Option<String>,
    /// The response if successful
    pub response: Option<LlmResponse>,
}

/// Result of executing a chain
#[derive(Debug)]
pub struct ChainResult {
    /// Whether the chain succeeded
    pub success: bool,
    /// The final response if successful
    pub response: Option<LlmResponse>,
    /// Results from each attempted step
    pub step_results: Vec<StepResult>,
    /// Total latency in milliseconds
    pub total_latency_ms: u64,
    /// Error message if failed
    pub error: Option<String>,
}

/// Provider resolver trait - resolves model IDs to LLM providers
#[async_trait]
pub trait ProviderResolver: Send + Sync {
    async fn resolve(&self, model_id: &ModelId) -> Result<Arc<dyn LlmProvider>, DomainError>;

    /// Get the provider-specific model name for a model ID
    async fn get_provider_model(&self, model_id: &ModelId) -> Result<String, DomainError>;
}

/// Chain executor - executes model chains with retry, fallback, and circuit breaker
pub struct ChainExecutor<R: ProviderResolver> {
    resolver: R,
    config: ChainExecutorConfig,
    circuit_breakers: RwLock<HashMap<String, Arc<CircuitBreaker>>>,
    metrics: RwLock<ChainMetrics>,
}

impl<R: ProviderResolver> std::fmt::Debug for ChainExecutor<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChainExecutor")
            .field("config", &self.config)
            .finish()
    }
}

impl<R: ProviderResolver> ChainExecutor<R> {
    /// Create a new chain executor
    pub fn new(resolver: R, config: ChainExecutorConfig) -> Self {
        Self {
            resolver,
            config,
            circuit_breakers: RwLock::new(HashMap::new()),
            metrics: RwLock::new(ChainMetrics::default()),
        }
    }

    /// Execute a model chain
    pub async fn execute(
        &self,
        chain: &ModelChain,
        request: LlmRequest,
    ) -> Result<ChainResult, DomainError> {
        if !chain.is_enabled() {
            return Err(DomainError::validation(format!(
                "Chain '{}' is disabled",
                chain.id()
            )));
        }

        if chain.is_empty() {
            return Err(DomainError::validation(format!(
                "Chain '{}' has no steps",
                chain.id()
            )));
        }

        let start = Instant::now();
        let mut step_results = Vec::new();
        let mut final_response = None;
        let mut final_error = None;

        for step in chain.steps() {
            let step_result = self.execute_step(step, &request).await;
            let success = step_result.success;

            step_results.push(step_result);

            if success {
                final_response = step_results.last().and_then(|r| r.response.clone());
                break;
            }

            // Handle fallback behavior
            match step.fallback_behavior() {
                FallbackBehavior::Continue => continue,
                FallbackBehavior::Stop => {
                    final_error = step_results.last().and_then(|r| r.error.clone());
                    break;
                }
                FallbackBehavior::Skip => break,
            }
        }

        let total_latency_ms = start.elapsed().as_millis() as u64;
        let success = final_response.is_some();

        // Update metrics
        if self.config.collect_metrics {
            self.update_metrics(success, total_latency_ms, &step_results)
                .await;
        }

        Ok(ChainResult {
            success,
            response: final_response,
            step_results,
            total_latency_ms,
            error: final_error,
        })
    }

    /// Execute a single step with retry logic
    async fn execute_step(&self, step: &ChainStep, request: &LlmRequest) -> StepResult {
        let model_id = step.model_id().clone();
        let start = Instant::now();

        // Check circuit breaker
        let circuit_breaker = self.get_or_create_circuit_breaker(&model_id).await;

        if !circuit_breaker.is_available() {
            return StepResult {
                model_id,
                success: false,
                attempts: 0,
                latency_ms: 0,
                error: Some("Circuit breaker is open".to_string()),
                response: None,
            };
        }

        let retry_config = step.retry_config();
        let max_attempts = retry_config.max_retries + 1;
        let mut last_error = None;

        for attempt in 0..max_attempts {
            if attempt > 0 {
                let delay = retry_config.delay_for_attempt(attempt - 1);
                tokio::time::sleep(delay).await;
            }

            match self.try_execute_step(step, request).await {
                Ok(response) => {
                    circuit_breaker.record_success();

                    return StepResult {
                        model_id,
                        success: true,
                        attempts: attempt + 1,
                        latency_ms: start.elapsed().as_millis() as u64,
                        error: None,
                        response: Some(response),
                    };
                }
                Err(e) => {
                    last_error = Some(e.to_string());
                }
            }
        }

        // All attempts failed
        circuit_breaker.record_failure();

        StepResult {
            model_id,
            success: false,
            attempts: max_attempts,
            latency_ms: start.elapsed().as_millis() as u64,
            error: last_error,
            response: None,
        }
    }

    /// Try to execute a step once (with optional timeout)
    async fn try_execute_step(
        &self,
        step: &ChainStep,
        request: &LlmRequest,
    ) -> Result<LlmResponse, DomainError> {
        let model_id = step.model_id();

        // Resolve provider and model name
        let provider = self.resolver.resolve(model_id).await?;
        let provider_model = self.resolver.get_provider_model(model_id).await?;

        // Execute with optional timeout
        let future = provider.chat(&provider_model, request.clone());

        if let Some(max_latency) = step.max_latency() {
            match timeout(max_latency, future).await {
                Ok(result) => result,
                Err(_) => Err(DomainError::provider(
                    provider.provider_name(),
                    format!("Request timed out after {}ms", step.max_latency_ms()),
                )),
            }
        } else {
            future.await
        }
    }

    /// Get or create a circuit breaker for a model
    async fn get_or_create_circuit_breaker(&self, model_id: &ModelId) -> Arc<CircuitBreaker> {
        let key = model_id.as_str().to_string();

        // Try to get existing
        {
            let breakers = self.circuit_breakers.read().await;

            if let Some(cb) = breakers.get(&key) {
                return cb.clone();
            }
        }

        // Create new
        let cb = Arc::new(CircuitBreaker::new(
            self.config.circuit_breaker_threshold,
            self.config.circuit_breaker_reset_ms,
        ));

        let mut breakers = self.circuit_breakers.write().await;
        breakers.insert(key, cb.clone());
        cb
    }

    /// Update metrics after chain execution
    async fn update_metrics(
        &self,
        success: bool,
        latency_ms: u64,
        step_results: &[StepResult],
    ) {
        let mut metrics = self.metrics.write().await;
        metrics.total_requests += 1;

        if success {
            metrics.successful_requests += 1;
        } else {
            metrics.failed_requests += 1;
        }

        metrics.total_latency_ms += latency_ms;

        // Update per-step metrics
        for result in step_results {
            let key = result.model_id.as_str().to_string();
            let step_metrics = metrics.step_metrics.entry(key).or_default();
            step_metrics.attempts += 1;

            if result.success {
                step_metrics.successes += 1;
            } else {
                step_metrics.failures += 1;
            }

            step_metrics.retries += result.attempts.saturating_sub(1) as u64;

            // Update average latency
            let total_latency =
                step_metrics.avg_latency_ms * (step_metrics.attempts - 1) as f64
                    + result.latency_ms as f64;
            step_metrics.avg_latency_ms = total_latency / step_metrics.attempts as f64;
        }
    }

    /// Get current metrics
    pub async fn get_metrics(&self) -> ChainMetrics {
        self.metrics.read().await.clone()
    }

    /// Reset metrics
    pub async fn reset_metrics(&self) {
        let mut metrics = self.metrics.write().await;
        *metrics = ChainMetrics::default();
    }

    /// Get circuit breaker state for a model
    pub async fn get_circuit_state(&self, model_id: &ModelId) -> CircuitState {
        let breakers = self.circuit_breakers.read().await;

        breakers
            .get(model_id.as_str())
            .map(|cb| cb.state())
            .unwrap_or(CircuitState::Closed)
    }

    /// Reset circuit breaker for a model
    pub async fn reset_circuit_breaker(&self, model_id: &ModelId) {
        let breakers = self.circuit_breakers.read().await;

        if let Some(cb) = breakers.get(model_id.as_str()) {
            cb.record_success(); // Resets failure count
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{FinishReason, Message, Usage};
    use std::sync::atomic::AtomicUsize;

    // Mock provider resolver for testing
    struct MockResolver {
        responses: RwLock<HashMap<String, Result<LlmResponse, String>>>,
        call_count: AtomicUsize,
    }

    impl MockResolver {
        fn new() -> Self {
            Self {
                responses: RwLock::new(HashMap::new()),
                call_count: AtomicUsize::new(0),
            }
        }

        async fn set_response(&self, model_id: &str, response: Result<LlmResponse, String>) {
            self.responses
                .write()
                .await
                .insert(model_id.to_string(), response);
        }

        fn call_count(&self) -> usize {
            self.call_count.load(Ordering::Relaxed)
        }
    }

    #[async_trait]
    impl ProviderResolver for MockResolver {
        async fn resolve(&self, model_id: &ModelId) -> Result<Arc<dyn LlmProvider>, DomainError> {
            self.call_count.fetch_add(1, Ordering::Relaxed);

            let responses = self.responses.read().await;

            if let Some(result) = responses.get(model_id.as_str()) {
                match result {
                    Ok(_) => Ok(Arc::new(MockProvider {
                        response: result.clone(),
                    })),
                    Err(e) => Err(DomainError::provider("mock", e.clone())),
                }
            } else {
                Err(DomainError::not_found(format!(
                    "Model '{}' not found",
                    model_id
                )))
            }
        }

        async fn get_provider_model(&self, model_id: &ModelId) -> Result<String, DomainError> {
            Ok(model_id.as_str().to_string())
        }
    }

    // Mock LLM provider
    #[derive(Debug)]
    struct MockProvider {
        response: Result<LlmResponse, String>,
    }

    #[async_trait]
    impl LlmProvider for MockProvider {
        async fn chat(
            &self,
            _model: &str,
            _request: LlmRequest,
        ) -> Result<LlmResponse, DomainError> {
            match &self.response {
                Ok(r) => Ok(r.clone()),
                Err(e) => Err(DomainError::provider("mock", e.clone())),
            }
        }

        async fn chat_stream(
            &self,
            _model: &str,
            _request: LlmRequest,
        ) -> Result<crate::domain::LlmStream, DomainError> {
            Err(DomainError::provider("mock", "Streaming not supported"))
        }

        fn provider_name(&self) -> &'static str {
            "mock"
        }

        fn available_models(&self) -> Vec<&'static str> {
            vec![]
        }
    }

    fn create_response(content: &str) -> LlmResponse {
        LlmResponse::new(
            "test-id".to_string(),
            "test-model".to_string(),
            Message::assistant(content),
        )
        .with_finish_reason(FinishReason::Stop)
        .with_usage(Usage::new(10, 20))
    }

    fn create_model_id(id: &str) -> ModelId {
        ModelId::new(id).unwrap()
    }

    fn create_chain_id(id: &str) -> super::super::ChainId {
        super::super::ChainId::new(id).unwrap()
    }

    #[tokio::test]
    async fn test_successful_chain_execution() {
        let resolver = MockResolver::new();
        resolver
            .set_response("model-1", Ok(create_response("Hello!")))
            .await;

        let executor = ChainExecutor::new(resolver, ChainExecutorConfig::default());

        let chain = ModelChain::new(create_chain_id("test-chain"), "Test Chain")
            .with_step(ChainStep::new(create_model_id("model-1")));

        let request = LlmRequest::builder().user("Hi").build();
        let result = executor.execute(&chain, request).await.unwrap();

        assert!(result.success);
        assert!(result.response.is_some());
        assert_eq!(result.step_results.len(), 1);
        assert!(result.step_results[0].success);
    }

    #[tokio::test]
    async fn test_fallback_to_second_model() {
        let resolver = MockResolver::new();
        resolver
            .set_response("model-1", Err("Primary failed".to_string()))
            .await;
        resolver
            .set_response("model-2", Ok(create_response("Fallback response")))
            .await;

        let config = ChainExecutorConfig {
            circuit_breaker_threshold: 10, // High threshold to avoid tripping
            ..Default::default()
        };
        let executor = ChainExecutor::new(resolver, config);

        let chain = ModelChain::new(create_chain_id("fallback-chain"), "Fallback Chain")
            .with_step(
                ChainStep::new(create_model_id("model-1"))
                    .with_max_retries(0)
                    .with_fallback_behavior(FallbackBehavior::Continue),
            )
            .with_step(ChainStep::new(create_model_id("model-2")));

        let request = LlmRequest::builder().user("Hi").build();
        let result = executor.execute(&chain, request).await.unwrap();

        assert!(result.success);
        assert_eq!(result.step_results.len(), 2);
        assert!(!result.step_results[0].success);
        assert!(result.step_results[1].success);
    }

    #[tokio::test]
    async fn test_stop_on_failure() {
        let resolver = MockResolver::new();
        resolver
            .set_response("model-1", Err("Failed".to_string()))
            .await;
        resolver
            .set_response("model-2", Ok(create_response("Should not reach")))
            .await;

        let config = ChainExecutorConfig {
            circuit_breaker_threshold: 10,
            ..Default::default()
        };
        let executor = ChainExecutor::new(resolver, config);

        let chain = ModelChain::new(create_chain_id("stop-chain"), "Stop Chain")
            .with_step(
                ChainStep::new(create_model_id("model-1"))
                    .with_max_retries(0)
                    .with_fallback_behavior(FallbackBehavior::Stop),
            )
            .with_step(ChainStep::new(create_model_id("model-2")));

        let request = LlmRequest::builder().user("Hi").build();
        let result = executor.execute(&chain, request).await.unwrap();

        assert!(!result.success);
        assert_eq!(result.step_results.len(), 1); // Only first step attempted
    }

    #[tokio::test]
    async fn test_disabled_chain() {
        let resolver = MockResolver::new();
        let executor = ChainExecutor::new(resolver, ChainExecutorConfig::default());

        let chain = ModelChain::new(create_chain_id("disabled"), "Disabled Chain")
            .with_enabled(false);

        let request = LlmRequest::builder().user("Hi").build();
        let result = executor.execute(&chain, request).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_empty_chain() {
        let resolver = MockResolver::new();
        let executor = ChainExecutor::new(resolver, ChainExecutorConfig::default());

        let chain = ModelChain::new(create_chain_id("empty"), "Empty Chain");

        let request = LlmRequest::builder().user("Hi").build();
        let result = executor.execute(&chain, request).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_circuit_breaker_opens() {
        let resolver = MockResolver::new();
        resolver
            .set_response("model-1", Err("Always fails".to_string()))
            .await;

        let config = ChainExecutorConfig {
            circuit_breaker_threshold: 2,
            circuit_breaker_reset_ms: 60000,
            collect_metrics: true,
        };
        let executor = ChainExecutor::new(resolver, config);

        let chain = ModelChain::new(create_chain_id("cb-chain"), "CB Chain")
            .with_step(ChainStep::new(create_model_id("model-1")).with_max_retries(0));

        let request = LlmRequest::builder().user("Hi").build();

        // First two requests should attempt the model
        for _ in 0..2 {
            let _ = executor.execute(&chain, request.clone()).await;
        }

        // Circuit breaker should now be open
        let state = executor
            .get_circuit_state(&create_model_id("model-1"))
            .await;
        assert_eq!(state, CircuitState::Open);

        // Third request should fail immediately with circuit breaker open
        let result = executor.execute(&chain, request).await.unwrap();
        assert!(!result.success);
        assert_eq!(result.step_results[0].attempts, 0); // No attempts made
    }

    #[tokio::test]
    async fn test_metrics_collection() {
        let resolver = MockResolver::new();
        resolver
            .set_response("model-1", Ok(create_response("Success")))
            .await;

        let executor = ChainExecutor::new(
            resolver,
            ChainExecutorConfig {
                collect_metrics: true,
                ..Default::default()
            },
        );

        let chain = ModelChain::new(create_chain_id("metrics-chain"), "Metrics Chain")
            .with_step(ChainStep::new(create_model_id("model-1")));

        let request = LlmRequest::builder().user("Hi").build();

        for _ in 0..3 {
            let _ = executor.execute(&chain, request.clone()).await;
        }

        let metrics = executor.get_metrics().await;
        assert_eq!(metrics.total_requests, 3);
        assert_eq!(metrics.successful_requests, 3);
        assert_eq!(metrics.failed_requests, 0);
    }

    #[test]
    fn test_retry_delay_calculation() {
        let config = super::super::RetryConfig::new(5)
            .with_initial_delay(100)
            .with_backoff_multiplier(2.0)
            .with_max_delay(1000);

        assert_eq!(config.delay_for_attempt(0), Duration::from_millis(100));
        assert_eq!(config.delay_for_attempt(1), Duration::from_millis(200));
        assert_eq!(config.delay_for_attempt(2), Duration::from_millis(400));
        assert_eq!(config.delay_for_attempt(3), Duration::from_millis(800));
        assert_eq!(config.delay_for_attempt(4), Duration::from_millis(1000)); // Capped
    }
}
