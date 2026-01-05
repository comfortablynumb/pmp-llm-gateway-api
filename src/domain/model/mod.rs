//! Model domain - Model configuration and management

mod entity;
mod repository;
mod validation;

pub use entity::{Model, ModelConfig, ModelId};
pub use repository::in_memory::InMemoryModelRepository;
pub use repository::ModelRepository;
pub use validation::{validate_model_config, validate_model_id, ModelValidationError};

#[cfg(test)]
pub use repository::mock;
