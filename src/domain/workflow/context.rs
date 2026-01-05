//! Workflow execution context and variable resolution
//!
//! Supports variable references in workflow templates:
//! - `${request:field}` - Required request input field
//! - `${request:field:default}` - Optional with default value
//! - `${step:step-name:field}` - Required step output field
//! - `${step:step-name:field:default}` - Optional with default value

use std::collections::HashMap;

use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::Value;

use super::error::WorkflowError;

/// Regex for request variable: ${request:field} or ${request:field:default}
/// Field can include dots for nested access (e.g., user.profile.name)
static REQUEST_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\$\{request:([a-zA-Z0-9_.-]+)(?::([^}]*))?\}").unwrap()
});

/// Regex for step variable: ${step:name:field} or ${step:name:field:default}
/// Field can include dots for nested access (e.g., result.items.0.name)
static STEP_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\$\{step:([a-zA-Z0-9_-]+):([a-zA-Z0-9_.-]+)(?::([^}]*))?\}").unwrap()
});

/// Workflow execution context holding request input and step outputs
#[derive(Debug, Clone)]
pub struct WorkflowContext {
    /// Input provided when executing the workflow
    request_input: Value,

    /// Outputs from executed steps, keyed by step name
    step_outputs: HashMap<String, Value>,
}

impl WorkflowContext {
    /// Create a new context with the given request input
    pub fn new(request_input: Value) -> Self {
        Self {
            request_input,
            step_outputs: HashMap::new(),
        }
    }

    /// Get the request input
    pub fn request_input(&self) -> &Value {
        &self.request_input
    }

    /// Get step outputs
    pub fn step_outputs(&self) -> &HashMap<String, Value> {
        &self.step_outputs
    }

    /// Set the output for a step
    pub fn set_step_output(&mut self, step_name: impl Into<String>, output: Value) {
        self.step_outputs.insert(step_name.into(), output);
    }

    /// Get the output of a specific step
    pub fn get_step_output(&self, step_name: &str) -> Option<&Value> {
        self.step_outputs.get(step_name)
    }

    /// Resolve a single variable expression to a JSON value
    ///
    /// Supports:
    /// - `${request:field}` - Get field from request input
    /// - `${request:field:default}` - With default value
    /// - `${step:name:field}` - Get field from step output
    /// - `${step:name:field:default}` - With default value
    pub fn resolve_expression(&self, expression: &str) -> Result<Value, WorkflowError> {
        // Try request pattern first
        if let Some(caps) = REQUEST_PATTERN.captures(expression) {
            let field = caps.get(1).unwrap().as_str();
            let default = caps.get(2).map(|m| m.as_str());

            return self.resolve_request_field(field, default);
        }

        // Try step pattern
        if let Some(caps) = STEP_PATTERN.captures(expression) {
            let step_name = caps.get(1).unwrap().as_str();
            let field = caps.get(2).unwrap().as_str();
            let default = caps.get(3).map(|m| m.as_str());

            return self.resolve_step_field(step_name, field, default);
        }

        // Not a variable expression, return as-is
        Err(WorkflowError::variable_resolution(format!(
            "Invalid variable expression: {}",
            expression
        )))
    }

    /// Resolve all variable references in a template string
    ///
    /// Replaces all `${...}` patterns with their resolved values.
    /// Returns the resulting string with all variables substituted.
    pub fn resolve_string(&self, template: &str) -> Result<String, WorkflowError> {
        let mut result = template.to_string();

        // First resolve request variables
        for caps in REQUEST_PATTERN.captures_iter(template) {
            let full_match = caps.get(0).unwrap().as_str();
            let field = caps.get(1).unwrap().as_str();
            let default = caps.get(2).map(|m| m.as_str());

            let value = self.resolve_request_field(field, default)?;
            let value_str = value_to_string(&value);
            result = result.replace(full_match, &value_str);
        }

        // Then resolve step variables
        for caps in STEP_PATTERN.captures_iter(template) {
            let full_match = caps.get(0).unwrap().as_str();
            let step_name = caps.get(1).unwrap().as_str();
            let field = caps.get(2).unwrap().as_str();
            let default = caps.get(3).map(|m| m.as_str());

            let value = self.resolve_step_field(step_name, field, default)?;
            let value_str = value_to_string(&value);
            result = result.replace(full_match, &value_str);
        }

        Ok(result)
    }

    /// Resolve a request field
    fn resolve_request_field(
        &self,
        field: &str,
        default: Option<&str>,
    ) -> Result<Value, WorkflowError> {
        // Support nested field access with dots
        let value = get_nested_field(&self.request_input, field);

        match value {
            Some(v) if !v.is_null() => Ok(v.clone()),
            _ => {
                if let Some(default_value) = default {
                    Ok(parse_default_value(default_value))
                } else {
                    Err(WorkflowError::variable_resolution(format!(
                        "Required request field '{}' not found",
                        field
                    )))
                }
            }
        }
    }

    /// Resolve a step output field
    fn resolve_step_field(
        &self,
        step_name: &str,
        field: &str,
        default: Option<&str>,
    ) -> Result<Value, WorkflowError> {
        let step_output = self.step_outputs.get(step_name);

        match step_output {
            Some(output) => {
                let value = get_nested_field(output, field);

                match value {
                    Some(v) if !v.is_null() => Ok(v.clone()),
                    _ => {
                        if let Some(default_value) = default {
                            Ok(parse_default_value(default_value))
                        } else {
                            Err(WorkflowError::variable_resolution(format!(
                                "Required field '{}' not found in step '{}'",
                                field, step_name
                            )))
                        }
                    }
                }
            }
            None => {
                if let Some(default_value) = default {
                    Ok(parse_default_value(default_value))
                } else {
                    Err(WorkflowError::variable_resolution(format!(
                        "Step '{}' output not found",
                        step_name
                    )))
                }
            }
        }
    }

    /// Check if a string contains any variable references
    pub fn has_variables(template: &str) -> bool {
        REQUEST_PATTERN.is_match(template) || STEP_PATTERN.is_match(template)
    }

    /// Extract all variable references from a template
    pub fn extract_variables(template: &str) -> Vec<VariableRef> {
        let mut variables = Vec::new();

        for caps in REQUEST_PATTERN.captures_iter(template) {
            let field = caps.get(1).unwrap().as_str().to_string();
            let default = caps.get(2).map(|m| m.as_str().to_string());

            variables.push(VariableRef::Request { field, default });
        }

        for caps in STEP_PATTERN.captures_iter(template) {
            let step = caps.get(1).unwrap().as_str().to_string();
            let field = caps.get(2).unwrap().as_str().to_string();
            let default = caps.get(3).map(|m| m.as_str().to_string());

            variables.push(VariableRef::Step {
                step,
                field,
                default,
            });
        }

        variables
    }
}

/// A parsed variable reference
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VariableRef {
    /// Reference to request input field
    Request {
        field: String,
        default: Option<String>,
    },

    /// Reference to step output field
    Step {
        step: String,
        field: String,
        default: Option<String>,
    },
}

impl VariableRef {
    /// Check if this variable has a default value
    pub fn has_default(&self) -> bool {
        match self {
            Self::Request { default, .. } => default.is_some(),
            Self::Step { default, .. } => default.is_some(),
        }
    }

    /// Check if this variable is required (no default)
    pub fn is_required(&self) -> bool {
        !self.has_default()
    }
}

/// Get a nested field from a JSON value using dot notation
fn get_nested_field<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = value;

    for part in parts {
        match current {
            Value::Object(obj) => {
                current = obj.get(part)?;
            }
            Value::Array(arr) => {
                let index: usize = part.parse().ok()?;
                current = arr.get(index)?;
            }
            _ => return None,
        }
    }

    Some(current)
}

/// Parse a default value string into a JSON value
fn parse_default_value(default: &str) -> Value {
    // Try to parse as JSON first
    if let Ok(parsed) = serde_json::from_str::<Value>(default) {
        return parsed;
    }

    // Otherwise treat as string
    Value::String(default.to_string())
}

/// Convert a JSON value to a string representation
fn value_to_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),

        // For arrays and objects, use JSON representation
        _ => serde_json::to_string(value).unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_context_creation() {
        let input = json!({"question": "What is Rust?"});
        let ctx = WorkflowContext::new(input.clone());

        assert_eq!(ctx.request_input(), &input);
        assert!(ctx.step_outputs().is_empty());
    }

    #[test]
    fn test_set_step_output() {
        let mut ctx = WorkflowContext::new(json!({}));
        ctx.set_step_output("search", json!({"documents": [{"id": 1}]}));

        assert!(ctx.get_step_output("search").is_some());
        assert!(ctx.get_step_output("other").is_none());
    }

    #[test]
    fn test_resolve_request_field() {
        let ctx = WorkflowContext::new(json!({
            "question": "What is Rust?",
            "options": {
                "limit": 10
            }
        }));

        // Simple field
        let result = ctx.resolve_expression("${request:question}").unwrap();
        assert_eq!(result, json!("What is Rust?"));

        // Nested field
        let result = ctx.resolve_expression("${request:options.limit}").unwrap();
        assert_eq!(result, json!(10));
    }

    #[test]
    fn test_resolve_request_field_with_default() {
        let ctx = WorkflowContext::new(json!({"question": "test"}));

        // Missing field with default
        let result = ctx.resolve_expression("${request:missing:default-value}").unwrap();
        assert_eq!(result, json!("default-value"));

        // Existing field ignores default
        let result = ctx.resolve_expression("${request:question:ignored}").unwrap();
        assert_eq!(result, json!("test"));
    }

    #[test]
    fn test_resolve_request_field_missing_required() {
        let ctx = WorkflowContext::new(json!({}));

        let result = ctx.resolve_expression("${request:missing}");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_resolve_step_field() {
        let mut ctx = WorkflowContext::new(json!({}));
        ctx.set_step_output("search", json!({
            "documents": [
                {"id": 1, "text": "First doc"},
                {"id": 2, "text": "Second doc"}
            ],
            "count": 2
        }));

        // Simple field
        let result = ctx.resolve_expression("${step:search:count}").unwrap();
        assert_eq!(result, json!(2));

        // Nested field
        let result = ctx.resolve_expression("${step:search:documents.0.text}").unwrap();
        assert_eq!(result, json!("First doc"));
    }

    #[test]
    fn test_resolve_step_field_with_default() {
        let mut ctx = WorkflowContext::new(json!({}));
        ctx.set_step_output("search", json!({"count": 5}));

        // Missing field with default
        let result = ctx.resolve_expression("${step:search:missing:0}").unwrap();
        assert_eq!(result, json!(0));

        // Missing step with default
        let result = ctx.resolve_expression("${step:other:field:fallback}").unwrap();
        assert_eq!(result, json!("fallback"));
    }

    #[test]
    fn test_resolve_step_field_missing_required() {
        let ctx = WorkflowContext::new(json!({}));

        let result = ctx.resolve_expression("${step:search:documents}");
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_string_simple() {
        let ctx = WorkflowContext::new(json!({"name": "Alice"}));

        let result = ctx.resolve_string("Hello, ${request:name}!").unwrap();
        assert_eq!(result, "Hello, Alice!");
    }

    #[test]
    fn test_resolve_string_multiple_variables() {
        let mut ctx = WorkflowContext::new(json!({"question": "What is Rust?"}));
        ctx.set_step_output("search", json!({"summary": "A systems programming language"}));

        let template = "Question: ${request:question}\nAnswer: ${step:search:summary}";
        let result = ctx.resolve_string(template).unwrap();

        assert_eq!(
            result,
            "Question: What is Rust?\nAnswer: A systems programming language"
        );
    }

    #[test]
    fn test_resolve_string_with_defaults() {
        let ctx = WorkflowContext::new(json!({}));

        let template = "Value: ${request:field:default-value}";
        let result = ctx.resolve_string(template).unwrap();

        assert_eq!(result, "Value: default-value");
    }

    #[test]
    fn test_resolve_string_no_variables() {
        let ctx = WorkflowContext::new(json!({}));

        let template = "No variables here";
        let result = ctx.resolve_string(template).unwrap();

        assert_eq!(result, "No variables here");
    }

    #[test]
    fn test_has_variables() {
        assert!(WorkflowContext::has_variables("${request:field}"));
        assert!(WorkflowContext::has_variables("${step:name:field}"));
        assert!(WorkflowContext::has_variables("Hello ${request:name}!"));
        assert!(!WorkflowContext::has_variables("No variables"));
        assert!(!WorkflowContext::has_variables("$not_a_variable"));
    }

    #[test]
    fn test_extract_variables() {
        let template = "${request:question} and ${step:search:documents} with ${request:limit:10}";
        let vars = WorkflowContext::extract_variables(template);

        assert_eq!(vars.len(), 3);

        assert!(matches!(&vars[0], VariableRef::Request { field, default }
            if field == "question" && default.is_none()));

        assert!(matches!(&vars[1], VariableRef::Request { field, default }
            if field == "limit" && default == &Some("10".to_string())));

        assert!(matches!(&vars[2], VariableRef::Step { step, field, default }
            if step == "search" && field == "documents" && default.is_none()));
    }

    #[test]
    fn test_variable_ref_is_required() {
        let required = VariableRef::Request {
            field: "test".to_string(),
            default: None,
        };
        assert!(required.is_required());

        let optional = VariableRef::Request {
            field: "test".to_string(),
            default: Some("default".to_string()),
        };
        assert!(!optional.is_required());
    }

    #[test]
    fn test_default_value_parsing() {
        // JSON number
        let ctx = WorkflowContext::new(json!({}));
        let result = ctx.resolve_expression("${request:num:42}").unwrap();
        assert_eq!(result, json!(42));

        // JSON boolean
        let result = ctx.resolve_expression("${request:bool:true}").unwrap();
        assert_eq!(result, json!(true));

        // JSON array
        let result = ctx.resolve_expression("${request:arr:[]}").unwrap();
        assert_eq!(result, json!([]));

        // Plain string (not valid JSON)
        let result = ctx.resolve_expression("${request:str:hello world}").unwrap();
        assert_eq!(result, json!("hello world"));
    }

    #[test]
    fn test_nested_field_access() {
        let ctx = WorkflowContext::new(json!({
            "user": {
                "profile": {
                    "name": "Alice",
                    "settings": {
                        "theme": "dark"
                    }
                }
            },
            "items": [
                {"name": "first"},
                {"name": "second"}
            ]
        }));

        // Deep object access
        let result = ctx.resolve_expression("${request:user.profile.name}").unwrap();
        assert_eq!(result, json!("Alice"));

        let result = ctx
            .resolve_expression("${request:user.profile.settings.theme}")
            .unwrap();
        assert_eq!(result, json!("dark"));

        // Array index access
        let result = ctx.resolve_expression("${request:items.0.name}").unwrap();
        assert_eq!(result, json!("first"));

        let result = ctx.resolve_expression("${request:items.1.name}").unwrap();
        assert_eq!(result, json!("second"));
    }

    #[test]
    fn test_value_to_string_conversion() {
        let mut ctx = WorkflowContext::new(json!({
            "string": "hello",
            "number": 42,
            "bool": true,
            "array": [1, 2, 3],
            "object": {"key": "value"}
        }));
        ctx.set_step_output("test", json!({"result": "done"}));

        // String stays as string
        let result = ctx.resolve_string("${request:string}").unwrap();
        assert_eq!(result, "hello");

        // Number converts to string
        let result = ctx.resolve_string("Count: ${request:number}").unwrap();
        assert_eq!(result, "Count: 42");

        // Bool converts to string
        let result = ctx.resolve_string("Active: ${request:bool}").unwrap();
        assert_eq!(result, "Active: true");

        // Array becomes JSON string
        let result = ctx.resolve_string("Items: ${request:array}").unwrap();
        assert_eq!(result, "Items: [1,2,3]");

        // Object becomes JSON string
        let result = ctx.resolve_string("Data: ${request:object}").unwrap();
        assert_eq!(result, "Data: {\"key\":\"value\"}");
    }
}
