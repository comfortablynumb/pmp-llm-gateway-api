//! Prompt entity and related types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::{validate_model_id, ModelValidationError};

/// Prompt identifier - uses same validation as ModelId
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct PromptId(String);

impl PromptId {
    /// Create a new PromptId after validation
    pub fn new(id: impl Into<String>) -> Result<Self, ModelValidationError> {
        let id = id.into();
        validate_model_id(&id)?;
        Ok(Self(id))
    }

    /// Get the inner string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for PromptId {
    type Error = ModelValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<PromptId> for String {
    fn from(id: PromptId) -> Self {
        id.0
    }
}

impl std::fmt::Display for PromptId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A versioned snapshot of a prompt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptVersion {
    /// Version number (1-indexed)
    version: u32,
    /// Content at this version
    content: String,
    /// When this version was created
    created_at: DateTime<Utc>,
    /// Optional commit message describing the change
    message: Option<String>,
}

impl PromptVersion {
    /// Create a new prompt version
    pub fn new(version: u32, content: impl Into<String>) -> Self {
        Self {
            version,
            content: content.into(),
            created_at: Utc::now(),
            message: None,
        }
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }
}

/// Prompt entity representing a reusable prompt template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prompt {
    /// Unique identifier
    id: PromptId,
    /// Display name
    name: String,
    /// Description of the prompt's purpose
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    /// Current prompt content (may contain variables)
    content: String,
    /// Current version number
    version: u32,
    /// Version history
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    history: Vec<PromptVersion>,
    /// Maximum versions to keep in history (0 = unlimited)
    max_history: usize,
    /// Whether the prompt is enabled
    enabled: bool,
    /// Tags for categorization
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    tags: Vec<String>,
    /// Creation timestamp
    created_at: DateTime<Utc>,
    /// Last update timestamp
    updated_at: DateTime<Utc>,
}

impl Prompt {
    /// Create a new Prompt with required fields
    pub fn new(id: PromptId, name: impl Into<String>, content: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id,
            name: name.into(),
            description: None,
            content: content.into(),
            version: 1,
            history: Vec::new(),
            max_history: 10, // Default to keeping 10 versions
            enabled: true,
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_max_history(mut self, max: usize) -> Self {
        self.max_history = max;
        self
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    // Getters

    pub fn id(&self) -> &PromptId {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn history(&self) -> &[PromptVersion] {
        &self.history
    }

    pub fn max_history(&self) -> usize {
        self.max_history
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    /// Get a specific version from history
    pub fn get_version(&self, version: u32) -> Option<&PromptVersion> {
        if version == self.version {
            return None; // Current version, not in history
        }
        self.history.iter().find(|v| v.version == version)
    }

    // Mutators

    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
        self.touch();
    }

    pub fn set_description(&mut self, description: Option<String>) {
        self.description = description;
        self.touch();
    }

    /// Update the content, creating a new version
    pub fn set_content(&mut self, content: impl Into<String>, message: Option<String>) {
        let new_content = content.into();

        // Don't create version if content is unchanged
        if new_content == self.content {
            return;
        }

        // Save current version to history
        let mut version = PromptVersion::new(self.version, self.content.clone());

        if let Some(msg) = message {
            version = version.with_message(msg);
        }

        self.history.push(version);

        // Trim history if needed
        if self.max_history > 0 && self.history.len() > self.max_history {
            let excess = self.history.len() - self.max_history;
            self.history.drain(0..excess);
        }

        // Update to new content
        self.content = new_content;
        self.version += 1;
        self.touch();
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        self.touch();
    }

    pub fn set_tags(&mut self, tags: Vec<String>) {
        self.tags = tags;
        self.touch();
    }

    pub fn add_tag(&mut self, tag: impl Into<String>) {
        let tag = tag.into();

        if !self.tags.contains(&tag) {
            self.tags.push(tag);
            self.touch();
        }
    }

    pub fn remove_tag(&mut self, tag: &str) -> bool {
        if let Some(pos) = self.tags.iter().position(|t| t == tag) {
            self.tags.remove(pos);
            self.touch();
            true
        } else {
            false
        }
    }

    /// Revert to a previous version
    pub fn revert_to_version(&mut self, version: u32) -> bool {
        if let Some(v) = self.history.iter().find(|v| v.version == version) {
            let old_content = v.content.clone();
            self.set_content(old_content, Some(format!("Reverted to version {}", version)));
            true
        } else {
            false
        }
    }

    fn touch(&mut self) {
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_prompt_id(id: &str) -> PromptId {
        PromptId::new(id).unwrap()
    }

    #[test]
    fn test_prompt_id_valid() {
        let id = PromptId::new("my-prompt-1").unwrap();
        assert_eq!(id.as_str(), "my-prompt-1");
    }

    #[test]
    fn test_prompt_id_invalid() {
        let result = PromptId::new("invalid_prompt!");
        assert!(result.is_err());
    }

    #[test]
    fn test_prompt_creation() {
        let prompt = Prompt::new(
            create_prompt_id("system-prompt"),
            "System Prompt",
            "You are a helpful assistant.",
        )
        .with_description("Default system prompt")
        .with_tag("system")
        .with_tag("default");

        assert_eq!(prompt.id().as_str(), "system-prompt");
        assert_eq!(prompt.name(), "System Prompt");
        assert_eq!(prompt.description(), Some("Default system prompt"));
        assert_eq!(prompt.content(), "You are a helpful assistant.");
        assert_eq!(prompt.version(), 1);
        assert!(prompt.history().is_empty());
        assert!(prompt.is_enabled());
        assert_eq!(prompt.tags(), &["system", "default"]);
    }

    #[test]
    fn test_prompt_versioning() {
        let mut prompt = Prompt::new(
            create_prompt_id("versioned"),
            "Versioned Prompt",
            "Version 1 content",
        );

        assert_eq!(prompt.version(), 1);
        assert!(prompt.history().is_empty());

        // Update content
        prompt.set_content("Version 2 content", Some("Updated for clarity".to_string()));
        assert_eq!(prompt.version(), 2);
        assert_eq!(prompt.content(), "Version 2 content");
        assert_eq!(prompt.history().len(), 1);
        assert_eq!(prompt.history()[0].version(), 1);
        assert_eq!(prompt.history()[0].content(), "Version 1 content");
        assert_eq!(prompt.history()[0].message(), Some("Updated for clarity"));

        // Update again
        prompt.set_content("Version 3 content", None);
        assert_eq!(prompt.version(), 3);
        assert_eq!(prompt.history().len(), 2);
    }

    #[test]
    fn test_prompt_version_unchanged() {
        let mut prompt = Prompt::new(
            create_prompt_id("unchanged"),
            "Unchanged",
            "Same content",
        );

        prompt.set_content("Same content", None);
        assert_eq!(prompt.version(), 1); // Should not increment
        assert!(prompt.history().is_empty());
    }

    #[test]
    fn test_prompt_history_limit() {
        let mut prompt = Prompt::new(
            create_prompt_id("limited"),
            "Limited History",
            "Initial",
        )
        .with_max_history(3);

        for i in 2..=6 {
            prompt.set_content(format!("Version {}", i), None);
        }

        assert_eq!(prompt.version(), 6);
        assert_eq!(prompt.history().len(), 3); // Only keeps 3 versions
        assert_eq!(prompt.history()[0].version(), 3); // Oldest kept is version 3
    }

    #[test]
    fn test_prompt_revert() {
        let mut prompt = Prompt::new(
            create_prompt_id("revertable"),
            "Revertable",
            "Version 1",
        );

        prompt.set_content("Version 2", None);
        prompt.set_content("Version 3", None);

        assert_eq!(prompt.version(), 3);

        // Revert to version 1
        let reverted = prompt.revert_to_version(1);
        assert!(reverted);
        assert_eq!(prompt.version(), 4); // New version created
        assert_eq!(prompt.content(), "Version 1"); // Content from version 1
    }

    #[test]
    fn test_prompt_tags() {
        let mut prompt = Prompt::new(
            create_prompt_id("tagged"),
            "Tagged",
            "Content",
        );

        prompt.add_tag("tag1");
        prompt.add_tag("tag2");
        prompt.add_tag("tag1"); // Duplicate, should not add

        assert_eq!(prompt.tags().len(), 2);

        let removed = prompt.remove_tag("tag1");
        assert!(removed);
        assert_eq!(prompt.tags(), &["tag2"]);

        let not_found = prompt.remove_tag("nonexistent");
        assert!(!not_found);
    }

    #[test]
    fn test_prompt_version_struct() {
        let version = PromptVersion::new(5, "Test content")
            .with_message("Test update");

        assert_eq!(version.version(), 5);
        assert_eq!(version.content(), "Test content");
        assert_eq!(version.message(), Some("Test update"));
    }
}
