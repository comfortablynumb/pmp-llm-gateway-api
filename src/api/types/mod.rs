//! OpenAI-compatible API types
//!
//! These types mirror the OpenAI API format for compatibility.

pub mod chat;
pub mod error;
pub mod json;
pub mod models;
pub mod operation;

pub use chat::{
    ChatCompletionChoice, ChatCompletionRequest, ChatCompletionResponse,
    ChatCompletionStreamChoice, ChatCompletionStreamResponse, ChatMessage, ChatMessageRole,
    ContentPart, DeltaContent, FinishReason, FunctionCall, MessageContent, StreamOptions,
    StopSequence, ToolCall, Usage,
};
pub use error::{ApiError, ApiErrorResponse};
pub use json::Json;
pub use models::{Model as ApiModel, ModelsResponse};
pub use operation::{
    AsyncOperationCreated, AsyncQueryParams, OperationResponse, OperationsListResponse,
    OperationsQueryParams,
};
