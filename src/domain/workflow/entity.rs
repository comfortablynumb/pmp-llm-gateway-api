//! Workflow domain entity

use std::fmt;

use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

use super::error::WorkflowError;
use super::step_types::WorkflowStepType;
use crate::domain::storage::{StorageEntity, StorageKey};

/// Maximum length for workflow IDs
pub const MAX_ID_LENGTH: usize = 50;

/// Regex pattern for valid workflow IDs: alphanumeric and hyphens
static ID_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[a-zA-Z0-9][a-zA-Z0-9-]*[a-zA-Z0-9]$|^[a-zA-Z0-9]$").unwrap());

/// Validated workflow identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct WorkflowId(String);

impl WorkflowId {
    /// Create a new validated workflow ID
    pub fn new(id: impl Into<String>) -> Result<Self, WorkflowError> {
        let id = id.into();
        validate_workflow_id(&id)?;
        Ok(Self(id))
    }

    /// Get the ID as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for WorkflowId {
    type Error = WorkflowError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<WorkflowId> for String {
    fn from(id: WorkflowId) -> Self {
        id.0
    }
}

impl fmt::Display for WorkflowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for WorkflowId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl StorageKey for WorkflowId {
    fn as_str(&self) -> &str {
        &self.0
    }
}

/// Validate a workflow ID string
pub fn validate_workflow_id(id: &str) -> Result<(), WorkflowError> {
    if id.is_empty() {
        return Err(WorkflowError::validation("Workflow ID cannot be empty"));
    }

    if id.len() > MAX_ID_LENGTH {
        return Err(WorkflowError::validation(format!(
            "Workflow ID exceeds maximum length of {} characters",
            MAX_ID_LENGTH
        )));
    }

    if !ID_PATTERN.is_match(id) {
        return Err(WorkflowError::validation(format!(
            "Invalid workflow ID '{}': must be alphanumeric with hyphens, start and end with alphanumeric",
            id
        )));
    }

    Ok(())
}

/// Action to take when a step fails
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum OnErrorAction {
    /// Stop the workflow and return an error
    #[default]
    FailWorkflow,

    /// Skip this step and continue to the next
    SkipStep,
}

/// A step within a workflow (wrapper around step type with metadata)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkflowStep {
    /// Unique name for this step within the workflow
    name: String,

    /// The step type and configuration
    #[serde(flatten)]
    step_type: WorkflowStepType,

    /// Optional JSON Schema for expected output validation
    #[serde(skip_serializing_if = "Option::is_none")]
    output_schema: Option<serde_json::Value>,

    /// Action to take on error
    #[serde(default)]
    on_error: OnErrorAction,

    /// Optional timeout in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    timeout_ms: Option<u64>,
}

impl WorkflowStep {
    /// Create a new workflow step
    pub fn new(name: impl Into<String>, step_type: WorkflowStepType) -> Self {
        Self {
            name: name.into(),
            step_type,
            output_schema: None,
            on_error: OnErrorAction::default(),
            timeout_ms: None,
        }
    }

    /// Set output schema for validation
    pub fn with_output_schema(mut self, schema: serde_json::Value) -> Self {
        self.output_schema = Some(schema);
        self
    }

    /// Set error handling behavior
    pub fn with_on_error(mut self, action: OnErrorAction) -> Self {
        self.on_error = action;
        self
    }

    /// Set step timeout
    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    /// Get the step name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the step type
    pub fn step_type(&self) -> &WorkflowStepType {
        &self.step_type
    }

    /// Get the output schema
    pub fn output_schema(&self) -> Option<&serde_json::Value> {
        self.output_schema.as_ref()
    }

    /// Get the error handling action
    pub fn on_error(&self) -> OnErrorAction {
        self.on_error
    }

    /// Get the timeout in milliseconds
    pub fn timeout_ms(&self) -> Option<u64> {
        self.timeout_ms
    }
}

/// A workflow definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    /// Unique workflow identifier
    id: WorkflowId,

    /// Human-readable name
    name: String,

    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,

    /// Optional JSON Schema for validating input
    #[serde(skip_serializing_if = "Option::is_none")]
    input_schema: Option<serde_json::Value>,

    /// Ordered list of workflow steps
    steps: Vec<WorkflowStep>,

    /// Configuration version (increments on changes)
    version: u32,

    /// Whether the workflow is enabled
    enabled: bool,

    /// When the workflow was created
    created_at: DateTime<Utc>,

    /// When the workflow was last updated
    updated_at: DateTime<Utc>,
}

impl Workflow {
    /// Create a new workflow
    pub fn new(id: WorkflowId, name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id,
            name: name.into(),
            description: None,
            input_schema: None,
            steps: Vec::new(),
            version: 1,
            enabled: true,
            created_at: now,
            updated_at: now,
        }
    }

    // Builder methods

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_input_schema(mut self, schema: serde_json::Value) -> Self {
        self.input_schema = Some(schema);
        self
    }

    pub fn with_steps(mut self, steps: Vec<WorkflowStep>) -> Self {
        self.steps = steps;
        self
    }

    pub fn with_step(mut self, step: WorkflowStep) -> Self {
        self.steps.push(step);
        self
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    // Getters

    pub fn id(&self) -> &WorkflowId {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn input_schema(&self) -> Option<&serde_json::Value> {
        self.input_schema.as_ref()
    }

    pub fn steps(&self) -> &[WorkflowStep] {
        &self.steps
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    /// Get a step by name
    pub fn get_step(&self, name: &str) -> Option<&WorkflowStep> {
        self.steps.iter().find(|s| s.name() == name)
    }

    /// Get the index of a step by name
    pub fn get_step_index(&self, name: &str) -> Option<usize> {
        self.steps.iter().position(|s| s.name() == name)
    }

    // Setters (mutate and update timestamp)

    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
        self.touch();
    }

    pub fn set_description(&mut self, description: Option<String>) {
        self.description = description;
        self.touch();
    }

    pub fn set_input_schema(&mut self, schema: Option<serde_json::Value>) {
        self.input_schema = schema;
        self.touch();
    }

    pub fn set_steps(&mut self, steps: Vec<WorkflowStep>) {
        self.steps = steps;
        self.increment_version();
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        self.touch();
    }

    fn touch(&mut self) {
        self.updated_at = Utc::now();
    }

    fn increment_version(&mut self) {
        self.version += 1;
        self.touch();
    }
}

impl StorageEntity for Workflow {
    type Key = WorkflowId;

    fn key(&self) -> &Self::Key {
        &self.id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::workflow::step_types::{ChatCompletionStep, ConditionalAction, ConditionalStep};

    #[test]
    fn test_workflow_id_valid() {
        assert!(WorkflowId::new("my-workflow").is_ok());
        assert!(WorkflowId::new("workflow123").is_ok());
        assert!(WorkflowId::new("a").is_ok());
        assert!(WorkflowId::new("my-long-workflow-name-123").is_ok());
    }

    #[test]
    fn test_workflow_id_invalid() {
        assert!(WorkflowId::new("").is_err());
        assert!(WorkflowId::new("-invalid").is_err());
        assert!(WorkflowId::new("invalid-").is_err());
        assert!(WorkflowId::new("has spaces").is_err());
        assert!(WorkflowId::new("has_underscores").is_err());

        let long_id = "a".repeat(51);
        assert!(WorkflowId::new(long_id).is_err());
    }

    #[test]
    fn test_workflow_id_display() {
        let id = WorkflowId::new("test-workflow").unwrap();
        assert_eq!(id.to_string(), "test-workflow");
        assert_eq!(id.as_str(), "test-workflow");
    }

    #[test]
    fn test_workflow_id_serialization() {
        let id = WorkflowId::new("test-workflow").unwrap();
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"test-workflow\"");

        let deserialized: WorkflowId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn test_workflow_creation() {
        let id = WorkflowId::new("test").unwrap();
        let workflow = Workflow::new(id.clone(), "Test Workflow")
            .with_description("A test workflow")
            .with_enabled(true);

        assert_eq!(workflow.id().as_str(), "test");
        assert_eq!(workflow.name(), "Test Workflow");
        assert_eq!(workflow.description(), Some("A test workflow"));
        assert!(workflow.is_enabled());
        assert_eq!(workflow.version(), 1);
        assert!(workflow.is_empty());
    }

    #[test]
    fn test_workflow_with_steps() {
        let id = WorkflowId::new("multi-step").unwrap();
        let workflow = Workflow::new(id, "Multi-Step")
            .with_step(WorkflowStep::new(
                "step1",
                WorkflowStepType::ChatCompletion(ChatCompletionStep::new("gpt-4", "sys-prompt")),
            ))
            .with_step(WorkflowStep::new(
                "step2",
                WorkflowStepType::Conditional(
                    ConditionalStep::new(vec![]).with_default_action(ConditionalAction::Continue),
                ),
            ));

        assert_eq!(workflow.step_count(), 2);
        assert!(!workflow.is_empty());
        assert!(workflow.get_step("step1").is_some());
        assert!(workflow.get_step("step2").is_some());
        assert!(workflow.get_step("step3").is_none());
        assert_eq!(workflow.get_step_index("step2"), Some(1));
    }

    #[test]
    fn test_workflow_step_builder() {
        let step = WorkflowStep::new(
            "my-step",
            WorkflowStepType::ChatCompletion(ChatCompletionStep::new("gpt-4", "sys-prompt")),
        )
        .with_on_error(OnErrorAction::SkipStep)
        .with_timeout_ms(30000);

        assert_eq!(step.name(), "my-step");
        assert_eq!(step.on_error(), OnErrorAction::SkipStep);
        assert_eq!(step.timeout_ms(), Some(30000));
    }

    #[test]
    fn test_workflow_mutation_updates_timestamp() {
        let id = WorkflowId::new("mutable").unwrap();
        let mut workflow = Workflow::new(id, "Original");
        let original_updated = workflow.updated_at();

        std::thread::sleep(std::time::Duration::from_millis(10));
        workflow.set_name("Updated");

        assert!(workflow.updated_at() > original_updated);
        assert_eq!(workflow.name(), "Updated");
    }

    #[test]
    fn test_workflow_step_change_increments_version() {
        let id = WorkflowId::new("versioned").unwrap();
        let mut workflow = Workflow::new(id, "Versioned");
        assert_eq!(workflow.version(), 1);

        workflow.set_steps(vec![WorkflowStep::new(
            "new-step",
            WorkflowStepType::ChatCompletion(ChatCompletionStep::new("gpt-4", "sys-prompt")),
        )]);

        assert_eq!(workflow.version(), 2);
    }

    #[test]
    fn test_workflow_serialization() {
        let id = WorkflowId::new("serializable").unwrap();
        let workflow = Workflow::new(id, "Serializable Workflow")
            .with_description("Test description")
            .with_step(WorkflowStep::new(
                "chat",
                WorkflowStepType::ChatCompletion(ChatCompletionStep::new("gpt-4", "sys-prompt")),
            ));

        let json = serde_json::to_string_pretty(&workflow).unwrap();
        assert!(json.contains("\"id\": \"serializable\""));
        assert!(json.contains("\"name\": \"Serializable Workflow\""));
        assert!(json.contains("\"type\": \"chat_completion\""));

        let deserialized: Workflow = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id().as_str(), "serializable");
        assert_eq!(deserialized.step_count(), 1);
    }

    #[test]
    fn test_on_error_action_default() {
        assert_eq!(OnErrorAction::default(), OnErrorAction::FailWorkflow);
    }
}
