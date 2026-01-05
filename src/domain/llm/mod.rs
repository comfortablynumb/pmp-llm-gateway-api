//! LLM provider domain models and traits

mod message;
mod provider;
mod request;
mod response;

pub use message::{ContentPart, Message, MessageRole};
pub use provider::{LlmProvider, LlmStream};
pub use request::{LlmRequest, LlmRequestBuilder};
pub use response::{FinishReason, LlmResponse, StreamChunk, Usage};

#[cfg(test)]
pub use provider::mock::MockLlmProvider;
