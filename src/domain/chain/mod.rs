//! Model chain domain - Chain configuration and execution

mod entity;
mod executor;
mod repository;

pub use entity::{ChainId, ChainStep, FallbackBehavior, ModelChain, RetryConfig};
pub use executor::{ChainExecutor, ChainExecutorConfig, ChainResult, StepResult};
pub use repository::ChainRepository;

#[cfg(test)]
pub use repository::mock;
