//! Prompt management domain - Prompt templates with variable support

mod entity;
mod repository;
mod template;

pub use entity::{Prompt, PromptId, PromptVersion};
pub use repository::in_memory::InMemoryPromptRepository;
pub use repository::PromptRepository;
pub use template::{PromptTemplate, PromptVariable, TemplateError};

#[cfg(test)]
pub use repository::mock;
