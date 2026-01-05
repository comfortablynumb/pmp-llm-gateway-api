//! Prompt template parsing and rendering
//!
//! Supports variable syntax: `${var:variable-name:default-value}`
//! - `${var:name}` - Required variable, error if not provided
//! - `${var:name:default}` - Optional variable with default value

use std::collections::HashMap;

use once_cell::sync::Lazy;
use regex::Regex;
use thiserror::Error;

/// Regex to match variable patterns: ${var:name} or ${var:name:default}
static VARIABLE_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\$\{var:([a-zA-Z0-9][-a-zA-Z0-9]*)(?::([^}]*))?\}").unwrap()
});

/// Template processing errors
#[derive(Debug, Clone, Error, PartialEq)]
pub enum TemplateError {
    #[error("Missing required variable: {name}")]
    MissingVariable { name: String },

    #[error("Invalid variable name: {name}")]
    InvalidVariableName { name: String },

    #[error("Template parsing error: {message}")]
    ParseError { message: String },
}

/// A parsed variable from a template
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptVariable {
    /// Variable name
    pub name: String,
    /// Default value if provided
    pub default: Option<String>,
    /// Whether the variable is required (no default)
    pub required: bool,
}

impl PromptVariable {
    /// Create a required variable
    pub fn required(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            default: None,
            required: true,
        }
    }

    /// Create an optional variable with a default
    pub fn with_default(name: impl Into<String>, default: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            default: Some(default.into()),
            required: false,
        }
    }
}

/// A parsed prompt template
#[derive(Debug, Clone)]
pub struct PromptTemplate {
    /// Original template content
    content: String,
    /// Parsed variables
    variables: Vec<PromptVariable>,
}

impl PromptTemplate {
    /// Parse a template string and extract variables
    pub fn parse(content: impl Into<String>) -> Result<Self, TemplateError> {
        let content = content.into();
        let mut variables = Vec::new();
        let mut seen_names = std::collections::HashSet::new();

        for cap in VARIABLE_PATTERN.captures_iter(&content) {
            let name = cap.get(1).unwrap().as_str().to_string();

            // Skip duplicates
            if seen_names.contains(&name) {
                continue;
            }
            seen_names.insert(name.clone());

            let default = cap.get(2).map(|m| m.as_str().to_string());

            let variable = if let Some(default_value) = default {
                PromptVariable::with_default(&name, default_value)
            } else {
                PromptVariable::required(&name)
            };

            variables.push(variable);
        }

        Ok(Self { content, variables })
    }

    /// Get the original template content
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Get all parsed variables
    pub fn variables(&self) -> &[PromptVariable] {
        &self.variables
    }

    /// Get only required variables (no default value)
    pub fn required_variables(&self) -> Vec<&PromptVariable> {
        self.variables.iter().filter(|v| v.required).collect()
    }

    /// Check if the template has any variables
    pub fn has_variables(&self) -> bool {
        !self.variables.is_empty()
    }

    /// Render the template with provided values
    pub fn render(&self, values: &HashMap<String, String>) -> Result<String, TemplateError> {
        let mut result = self.content.clone();

        for var in &self.variables {
            let value = values.get(&var.name).or(var.default.as_ref());

            match value {
                Some(v) => {
                    // Build the pattern to replace
                    let pattern = if let Some(ref default) = var.default {
                        format!("${{var:{}:{}}}", var.name, default)
                    } else {
                        format!("${{var:{}}}", var.name)
                    };
                    result = result.replace(&pattern, v);
                }
                None => {
                    return Err(TemplateError::MissingVariable {
                        name: var.name.clone(),
                    });
                }
            }
        }

        Ok(result)
    }

    /// Render the template, using defaults for missing values
    pub fn render_with_defaults(&self, values: &HashMap<String, String>) -> Result<String, TemplateError> {
        // Check that all required variables are provided
        for var in &self.variables {
            if var.required && !values.contains_key(&var.name) {
                return Err(TemplateError::MissingVariable {
                    name: var.name.clone(),
                });
            }
        }

        self.render(values)
    }
}

/// Convenience function to render a template string directly
pub fn render_template(
    template: &str,
    values: &HashMap<String, String>,
) -> Result<String, TemplateError> {
    let parsed = PromptTemplate::parse(template)?;
    parsed.render(values)
}

/// Extract variable names from a template string
pub fn extract_variables(template: &str) -> Vec<PromptVariable> {
    match PromptTemplate::parse(template) {
        Ok(t) => t.variables,
        Err(_) => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_no_variables() {
        let template = PromptTemplate::parse("Hello, world!").unwrap();
        assert!(!template.has_variables());
        assert!(template.variables().is_empty());
    }

    #[test]
    fn test_parse_required_variable() {
        let template = PromptTemplate::parse("Hello, ${var:name}!").unwrap();
        assert!(template.has_variables());
        assert_eq!(template.variables().len(), 1);

        let var = &template.variables()[0];
        assert_eq!(var.name, "name");
        assert!(var.required);
        assert!(var.default.is_none());
    }

    #[test]
    fn test_parse_variable_with_default() {
        let template = PromptTemplate::parse("Hello, ${var:name:World}!").unwrap();
        assert_eq!(template.variables().len(), 1);

        let var = &template.variables()[0];
        assert_eq!(var.name, "name");
        assert!(!var.required);
        assert_eq!(var.default, Some("World".to_string()));
    }

    #[test]
    fn test_parse_multiple_variables() {
        let template = PromptTemplate::parse(
            "Hello ${var:greeting:Hi}, ${var:name}! You are ${var:role:assistant}.",
        )
        .unwrap();

        assert_eq!(template.variables().len(), 3);
        assert_eq!(template.required_variables().len(), 1);
    }

    #[test]
    fn test_parse_duplicate_variables() {
        let template =
            PromptTemplate::parse("${var:name} and ${var:name} again").unwrap();

        // Should only have one variable
        assert_eq!(template.variables().len(), 1);
    }

    #[test]
    fn test_render_required_variable() {
        let template = PromptTemplate::parse("Hello, ${var:name}!").unwrap();

        let mut values = HashMap::new();
        values.insert("name".to_string(), "Alice".to_string());

        let result = template.render(&values).unwrap();
        assert_eq!(result, "Hello, Alice!");
    }

    #[test]
    fn test_render_missing_required_variable() {
        let template = PromptTemplate::parse("Hello, ${var:name}!").unwrap();
        let values = HashMap::new();

        let result = template.render(&values);
        assert!(result.is_err());

        match result {
            Err(TemplateError::MissingVariable { name }) => {
                assert_eq!(name, "name");
            }
            _ => panic!("Expected MissingVariable error"),
        }
    }

    #[test]
    fn test_render_with_default() {
        let template = PromptTemplate::parse("Hello, ${var:name:World}!").unwrap();
        let values = HashMap::new();

        let result = template.render(&values).unwrap();
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_render_override_default() {
        let template = PromptTemplate::parse("Hello, ${var:name:World}!").unwrap();

        let mut values = HashMap::new();
        values.insert("name".to_string(), "Alice".to_string());

        let result = template.render(&values).unwrap();
        assert_eq!(result, "Hello, Alice!");
    }

    #[test]
    fn test_render_multiple_variables() {
        let template = PromptTemplate::parse(
            "You are ${var:role:an assistant}. Your name is ${var:name}.",
        )
        .unwrap();

        let mut values = HashMap::new();
        values.insert("name".to_string(), "Claude".to_string());

        let result = template.render(&values).unwrap();
        assert_eq!(result, "You are an assistant. Your name is Claude.");
    }

    #[test]
    fn test_render_empty_default() {
        let template = PromptTemplate::parse("Value: ${var:optional:}").unwrap();
        let values = HashMap::new();

        let result = template.render(&values).unwrap();
        assert_eq!(result, "Value: ");
    }

    #[test]
    fn test_render_complex_template() {
        let template = PromptTemplate::parse(
            r#"You are ${var:role:a helpful assistant}.

Your task is to ${var:task}.

Guidelines:
- Be ${var:tone:professional}
- Focus on ${var:focus:accuracy}

User context: ${var:context:No additional context provided}"#,
        )
        .unwrap();

        let mut values = HashMap::new();
        values.insert("task".to_string(), "answer questions".to_string());
        values.insert("tone".to_string(), "friendly".to_string());

        let result = template.render(&values).unwrap();

        assert!(result.contains("You are a helpful assistant"));
        assert!(result.contains("answer questions"));
        assert!(result.contains("Be friendly"));
        assert!(result.contains("Focus on accuracy"));
        assert!(result.contains("No additional context provided"));
    }

    #[test]
    fn test_variable_name_with_hyphens() {
        let template =
            PromptTemplate::parse("${var:user-name} and ${var:api-key:default-key}").unwrap();

        assert_eq!(template.variables().len(), 2);
        assert_eq!(template.variables()[0].name, "user-name");
        assert_eq!(template.variables()[1].name, "api-key");
    }

    #[test]
    fn test_convenience_render_function() {
        let mut values = HashMap::new();
        values.insert("name".to_string(), "World".to_string());

        let result = render_template("Hello, ${var:name}!", &values).unwrap();
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_extract_variables_function() {
        let vars = extract_variables("${var:a} ${var:b:default} ${var:c}");

        assert_eq!(vars.len(), 3);
        assert_eq!(vars[0].name, "a");
        assert!(vars[0].required);
        assert_eq!(vars[1].name, "b");
        assert!(!vars[1].required);
        assert_eq!(vars[2].name, "c");
        assert!(vars[2].required);
    }
}
