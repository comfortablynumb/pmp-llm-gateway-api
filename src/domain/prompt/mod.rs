//! Prompt management domain - Prompt templates with variable support

mod entity;
mod template;

pub use entity::{Prompt, PromptId, PromptVersion};
pub use template::{PromptTemplate, PromptVariable, TemplateError};
