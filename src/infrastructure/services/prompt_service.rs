//! Prompt service - CRUD operations and rendering for prompts

use std::collections::HashMap;
use std::sync::Arc;

use crate::domain::{
    DomainError, ModelValidationError, Prompt, PromptId, PromptRepository, PromptTemplate,
    TemplateError,
};

/// Request to create a new prompt
#[derive(Debug, Clone)]
pub struct CreatePromptRequest {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub content: String,
    pub tags: Vec<String>,
    pub enabled: bool,
    pub max_history: Option<usize>,
}

/// Request to update an existing prompt
#[derive(Debug, Clone)]
pub struct UpdatePromptRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub content: Option<String>,
    pub content_message: Option<String>,
    pub tags: Option<Vec<String>>,
    pub enabled: Option<bool>,
}

/// Request to render a prompt
#[derive(Debug, Clone)]
pub struct RenderPromptRequest {
    pub prompt_id: String,
    pub variables: HashMap<String, String>,
}

/// Rendered prompt result
#[derive(Debug, Clone)]
pub struct RenderedPrompt {
    pub prompt_id: String,
    pub version: u32,
    pub content: String,
    pub variables_used: Vec<String>,
}

/// Prompt service for CRUD and rendering operations
#[derive(Debug)]
pub struct PromptService<R: PromptRepository> {
    repository: Arc<R>,
}

impl<R: PromptRepository> PromptService<R> {
    /// Create a new PromptService with the given repository
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }

    /// Get a prompt by ID
    pub async fn get(&self, id: &str) -> Result<Option<Prompt>, DomainError> {
        let prompt_id = self.parse_prompt_id(id)?;
        self.repository.get(&prompt_id).await
    }

    /// Get a prompt by ID, returning an error if not found
    pub async fn get_required(&self, id: &str) -> Result<Prompt, DomainError> {
        self.get(id)
            .await?
            .ok_or_else(|| DomainError::not_found(format!("Prompt '{}' not found", id)))
    }

    /// List all prompts
    pub async fn list(&self) -> Result<Vec<Prompt>, DomainError> {
        self.repository.list().await
    }

    /// List all enabled prompts
    pub async fn list_enabled(&self) -> Result<Vec<Prompt>, DomainError> {
        self.repository.list_enabled().await
    }

    /// List prompts by tag
    pub async fn list_by_tag(&self, tag: &str) -> Result<Vec<Prompt>, DomainError> {
        self.repository.list_by_tag(tag).await
    }

    /// Create a new prompt
    pub async fn create(&self, request: CreatePromptRequest) -> Result<Prompt, DomainError> {
        let prompt_id = self.parse_prompt_id(&request.id)?;

        // Check for duplicate
        if self.repository.exists(&prompt_id).await? {
            return Err(DomainError::conflict(format!(
                "Prompt with ID '{}' already exists",
                request.id
            )));
        }

        // Validate template syntax
        self.validate_template(&request.content)?;

        // Build the prompt
        let mut prompt = Prompt::new(prompt_id, request.name, request.content);

        if let Some(description) = request.description {
            prompt = prompt.with_description(description);
        }

        if let Some(max_history) = request.max_history {
            prompt = prompt.with_max_history(max_history);
        }

        prompt = prompt.with_tags(request.tags).with_enabled(request.enabled);

        self.repository.create(prompt).await
    }

    /// Update an existing prompt
    pub async fn update(
        &self,
        id: &str,
        request: UpdatePromptRequest,
    ) -> Result<Prompt, DomainError> {
        let prompt_id = self.parse_prompt_id(id)?;

        // Get existing prompt
        let mut prompt = self
            .repository
            .get(&prompt_id)
            .await?
            .ok_or_else(|| DomainError::not_found(format!("Prompt '{}' not found", id)))?;

        // Apply updates
        if let Some(name) = request.name {
            prompt.set_name(name);
        }

        if let Some(description) = request.description {
            prompt.set_description(Some(description));
        }

        if let Some(content) = request.content {
            // Validate template syntax
            self.validate_template(&content)?;
            prompt.set_content(content, request.content_message);
        }

        if let Some(tags) = request.tags {
            prompt.set_tags(tags);
        }

        if let Some(enabled) = request.enabled {
            prompt.set_enabled(enabled);
        }

        self.repository.update(prompt).await
    }

    /// Delete a prompt by ID
    pub async fn delete(&self, id: &str) -> Result<bool, DomainError> {
        let prompt_id = self.parse_prompt_id(id)?;
        self.repository.delete(&prompt_id).await
    }

    /// Render a prompt with variables
    pub async fn render(&self, request: RenderPromptRequest) -> Result<RenderedPrompt, DomainError> {
        let prompt = self.get_required(&request.prompt_id).await?;

        if !prompt.is_enabled() {
            return Err(DomainError::validation(format!(
                "Prompt '{}' is disabled",
                request.prompt_id
            )));
        }

        let template = PromptTemplate::parse(prompt.content())
            .map_err(|e| DomainError::validation(e.to_string()))?;

        let variables_used: Vec<String> = template
            .variables()
            .iter()
            .map(|v| v.name.clone())
            .collect();

        let content = template
            .render(&request.variables)
            .map_err(|e| self.template_error_to_domain(e))?;

        Ok(RenderedPrompt {
            prompt_id: request.prompt_id,
            version: prompt.version(),
            content,
            variables_used,
        })
    }

    /// Render a prompt directly by ID with variables
    pub async fn render_by_id(
        &self,
        id: &str,
        variables: HashMap<String, String>,
    ) -> Result<String, DomainError> {
        let result = self
            .render(RenderPromptRequest {
                prompt_id: id.to_string(),
                variables,
            })
            .await?;

        Ok(result.content)
    }

    /// Get variables from a prompt
    pub async fn get_variables(&self, id: &str) -> Result<Vec<String>, DomainError> {
        let prompt = self.get_required(id).await?;

        let template = PromptTemplate::parse(prompt.content())
            .map_err(|e| DomainError::validation(e.to_string()))?;

        Ok(template
            .variables()
            .iter()
            .map(|v| v.name.clone())
            .collect())
    }

    /// Revert a prompt to a previous version
    pub async fn revert(&self, id: &str, version: u32) -> Result<Prompt, DomainError> {
        let prompt_id = self.parse_prompt_id(id)?;

        let mut prompt = self
            .repository
            .get(&prompt_id)
            .await?
            .ok_or_else(|| DomainError::not_found(format!("Prompt '{}' not found", id)))?;

        if !prompt.revert_to_version(version) {
            return Err(DomainError::not_found(format!(
                "Version {} not found in prompt history",
                version
            )));
        }

        self.repository.update(prompt).await
    }

    /// Enable a prompt
    pub async fn enable(&self, id: &str) -> Result<Prompt, DomainError> {
        self.update(
            id,
            UpdatePromptRequest {
                name: None,
                description: None,
                content: None,
                content_message: None,
                tags: None,
                enabled: Some(true),
            },
        )
        .await
    }

    /// Disable a prompt
    pub async fn disable(&self, id: &str) -> Result<Prompt, DomainError> {
        self.update(
            id,
            UpdatePromptRequest {
                name: None,
                description: None,
                content: None,
                content_message: None,
                tags: None,
                enabled: Some(false),
            },
        )
        .await
    }

    /// Parse and validate a prompt ID string
    fn parse_prompt_id(&self, id: &str) -> Result<PromptId, DomainError> {
        PromptId::new(id).map_err(|e| self.validation_error_to_domain(e))
    }

    /// Validate template syntax
    fn validate_template(&self, content: &str) -> Result<(), DomainError> {
        PromptTemplate::parse(content).map_err(|e| DomainError::validation(e.to_string()))?;
        Ok(())
    }

    /// Convert ModelValidationError to DomainError
    fn validation_error_to_domain(&self, error: ModelValidationError) -> DomainError {
        DomainError::validation(error.to_string())
    }

    /// Convert TemplateError to DomainError
    fn template_error_to_domain(&self, error: TemplateError) -> DomainError {
        match error {
            TemplateError::MissingVariable { name } => {
                DomainError::validation(format!("Missing required variable: {}", name))
            }
            _ => DomainError::validation(error.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::prompt::mock::MockPromptRepository;

    fn create_service() -> PromptService<MockPromptRepository> {
        PromptService::new(Arc::new(MockPromptRepository::new()))
    }

    fn create_request(id: &str) -> CreatePromptRequest {
        CreatePromptRequest {
            id: id.to_string(),
            name: format!("Test Prompt {}", id),
            description: Some("A test prompt".to_string()),
            content: "You are a helpful ${var:role:assistant}.".to_string(),
            tags: vec!["test".to_string()],
            enabled: true,
            max_history: Some(5),
        }
    }

    #[tokio::test]
    async fn test_create_prompt() {
        let service = create_service();
        let request = create_request("test-prompt");

        let prompt = service.create(request).await.unwrap();

        assert_eq!(prompt.id().as_str(), "test-prompt");
        assert_eq!(prompt.name(), "Test Prompt test-prompt");
        assert_eq!(prompt.max_history(), 5);
        assert!(prompt.is_enabled());
    }

    #[tokio::test]
    async fn test_create_duplicate_prompt() {
        let service = create_service();
        let request = create_request("duplicate");

        service.create(request.clone()).await.unwrap();
        let result = service.create(request).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_prompt_invalid_id() {
        let service = create_service();
        let request = CreatePromptRequest {
            id: "invalid_id!".to_string(),
            name: "Test".to_string(),
            description: None,
            content: "Content".to_string(),
            tags: vec![],
            enabled: true,
            max_history: None,
        };

        let result = service.create(request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_render_prompt() {
        let service = create_service();
        service.create(create_request("render-test")).await.unwrap();

        let mut variables = HashMap::new();
        variables.insert("role".to_string(), "expert".to_string());

        let rendered = service
            .render(RenderPromptRequest {
                prompt_id: "render-test".to_string(),
                variables,
            })
            .await
            .unwrap();

        assert_eq!(rendered.content, "You are a helpful expert.");
        assert_eq!(rendered.variables_used, vec!["role"]);
    }

    #[tokio::test]
    async fn test_render_prompt_with_default() {
        let service = create_service();
        service.create(create_request("default-test")).await.unwrap();

        let rendered = service
            .render(RenderPromptRequest {
                prompt_id: "default-test".to_string(),
                variables: HashMap::new(),
            })
            .await
            .unwrap();

        assert_eq!(rendered.content, "You are a helpful assistant.");
    }

    #[tokio::test]
    async fn test_render_disabled_prompt() {
        let service = create_service();
        service.create(create_request("disabled-test")).await.unwrap();
        service.disable("disabled-test").await.unwrap();

        let result = service
            .render(RenderPromptRequest {
                prompt_id: "disabled-test".to_string(),
                variables: HashMap::new(),
            })
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_prompt_content() {
        let service = create_service();
        service.create(create_request("update-test")).await.unwrap();

        let updated = service
            .update(
                "update-test",
                UpdatePromptRequest {
                    name: None,
                    description: None,
                    content: Some("New content: ${var:message}".to_string()),
                    content_message: Some("Updated template".to_string()),
                    tags: None,
                    enabled: None,
                },
            )
            .await
            .unwrap();

        assert_eq!(updated.version(), 2);
        assert_eq!(updated.content(), "New content: ${var:message}");
        assert_eq!(updated.history().len(), 1);
    }

    #[tokio::test]
    async fn test_get_variables() {
        let service = create_service();

        let request = CreatePromptRequest {
            id: "vars-test".to_string(),
            name: "Variables Test".to_string(),
            description: None,
            content: "${var:name}, you are a ${var:role:assistant} for ${var:task}.".to_string(),
            tags: vec![],
            enabled: true,
            max_history: None,
        };

        service.create(request).await.unwrap();

        let variables = service.get_variables("vars-test").await.unwrap();

        assert_eq!(variables.len(), 3);
        assert!(variables.contains(&"name".to_string()));
        assert!(variables.contains(&"role".to_string()));
        assert!(variables.contains(&"task".to_string()));
    }

    #[tokio::test]
    async fn test_list_by_tag() {
        let service = create_service();

        let mut request1 = create_request("tag-1");
        request1.tags = vec!["system".to_string()];
        service.create(request1).await.unwrap();

        let mut request2 = create_request("tag-2");
        request2.tags = vec!["user".to_string()];
        service.create(request2).await.unwrap();

        let system_prompts = service.list_by_tag("system").await.unwrap();
        assert_eq!(system_prompts.len(), 1);
        assert_eq!(system_prompts[0].id().as_str(), "tag-1");
    }

    #[tokio::test]
    async fn test_enable_disable() {
        let service = create_service();
        service.create(create_request("toggle-test")).await.unwrap();

        let disabled = service.disable("toggle-test").await.unwrap();
        assert!(!disabled.is_enabled());

        let enabled = service.enable("toggle-test").await.unwrap();
        assert!(enabled.is_enabled());
    }
}
