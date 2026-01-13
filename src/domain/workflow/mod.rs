//! Workflow domain module
//!
//! This module provides configurable multi-step workflows that chain operations together.
//! Workflows support:
//! - Chat completions with LLM models
//! - Knowledge base searches
//! - CRAG (Corrective RAG) document scoring
//! - Conditional branching
//!
//! ## Variable References
//!
//! Workflows support variable references using the following syntax:
//! - `${request:field}` - Reference to workflow execution request input
//! - `${request:field:default}` - With default value
//! - `${step:step-name:field}` - Reference to previous step output
//! - `${step:step-name:field:default}` - With default value

mod context;
mod entity;
mod error;
mod executor;
pub mod repository;
mod step_types;

pub use context::{VariableRef, WorkflowContext};
pub use entity::{
    validate_workflow_id, OnErrorAction, Workflow, WorkflowId, WorkflowStep, MAX_ID_LENGTH,
};
pub use error::WorkflowError;
pub use executor::{StepExecutionResult, WorkflowExecutor, WorkflowResult, WorkflowTokenUsage};
pub use repository::WorkflowRepository;
pub use step_types::{
    ChatCompletionStep, Condition, ConditionalAction, ConditionalStep, ConditionOperator,
    CragScoringStep, HttpMethod, HttpRequestStep, KnowledgeBaseSearchStep, ScoringStrategy,
    WorkflowStepType,
};
