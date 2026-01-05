use async_trait::async_trait;
use std::fmt::Debug;
use std::pin::Pin;
use futures::Stream;

use super::{LlmRequest, LlmResponse};
use super::response::StreamChunk;
use crate::domain::DomainError;

/// Stream type for LLM responses
pub type LlmStream = Pin<Box<dyn Stream<Item = Result<StreamChunk, DomainError>> + Send>>;

/// Trait for LLM providers (OpenAI, Anthropic, etc.)
#[async_trait]
pub trait LlmProvider: Send + Sync + Debug {
    /// Send a chat completion request
    async fn chat(&self, model: &str, request: LlmRequest) -> Result<LlmResponse, DomainError>;

    /// Send a streaming chat completion request
    async fn chat_stream(
        &self,
        model: &str,
        request: LlmRequest,
    ) -> Result<LlmStream, DomainError>;

    /// Get the provider name
    fn provider_name(&self) -> &'static str;

    /// List available models for this provider
    fn available_models(&self) -> Vec<&'static str>;
}

#[cfg(test)]
pub mod mock {
    use super::*;
    use futures::stream;

    #[derive(Debug)]
    pub struct MockLlmProvider {
        name: &'static str,
        response: Option<LlmResponse>,
        error: Option<String>,
    }

    impl MockLlmProvider {
        pub fn new(name: &'static str) -> Self {
            Self {
                name,
                response: None,
                error: None,
            }
        }

        pub fn with_response(mut self, response: LlmResponse) -> Self {
            self.response = Some(response);
            self
        }

        pub fn with_error(mut self, error: impl Into<String>) -> Self {
            self.error = Some(error.into());
            self
        }
    }

    #[async_trait]
    impl LlmProvider for MockLlmProvider {
        async fn chat(
            &self,
            _model: &str,
            _request: LlmRequest,
        ) -> Result<LlmResponse, DomainError> {
            if let Some(ref error) = self.error {
                return Err(DomainError::provider(self.name, error));
            }

            self.response
                .clone()
                .ok_or_else(|| DomainError::provider(self.name, "No mock response configured"))
        }

        async fn chat_stream(
            &self,
            model: &str,
            request: LlmRequest,
        ) -> Result<LlmStream, DomainError> {
            let response = self.chat(model, request).await?;
            let content = response.content().unwrap_or("").to_string();

            let chunks: Vec<Result<StreamChunk, DomainError>> = content
                .chars()
                .map(|c| {
                    Ok(StreamChunk::new(response.id.clone(), response.model.clone())
                        .with_delta(c.to_string()))
                })
                .chain(std::iter::once(Ok(
                    StreamChunk::new(response.id.clone(), response.model.clone())
                        .with_finish_reason(super::super::FinishReason::Stop),
                )))
                .collect();

            Ok(Box::pin(stream::iter(chunks)))
        }

        fn provider_name(&self) -> &'static str {
            self.name
        }

        fn available_models(&self) -> Vec<&'static str> {
            vec!["mock-model"]
        }
    }
}
