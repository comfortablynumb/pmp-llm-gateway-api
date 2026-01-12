//! Model domain - Model configuration and management

mod entity;
mod validation;

pub use entity::{Model, ModelConfig, ModelId};
pub use validation::{validate_model_config, validate_model_id, ModelValidationError};
