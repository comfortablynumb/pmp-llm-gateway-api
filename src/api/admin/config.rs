//! Configuration management admin endpoints

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::api::middleware::RequireAdmin;
use crate::api::state::AppState;
use crate::api::types::ApiError;
use crate::domain::{ConfigCategory, ConfigValue};

/// Configuration entry response
#[derive(Debug, Clone, Serialize)]
pub struct ConfigEntryResponse {
    pub key: String,
    pub value: ConfigValueResponse,
    pub category: String,
    pub description: Option<String>,
    pub updated_at: String,
}

/// Configuration value in response format
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum ConfigValueResponse {
    #[serde(rename = "string")]
    String(String),
    #[serde(rename = "integer")]
    Integer(i64),
    #[serde(rename = "float")]
    Float(f64),
    #[serde(rename = "boolean")]
    Boolean(bool),
    #[serde(rename = "string_list")]
    StringList(Vec<String>),
}

impl From<&ConfigValue> for ConfigValueResponse {
    fn from(value: &ConfigValue) -> Self {
        match value {
            ConfigValue::String(s) => ConfigValueResponse::String(s.clone()),
            ConfigValue::Integer(i) => ConfigValueResponse::Integer(*i),
            ConfigValue::Float(f) => ConfigValueResponse::Float(*f),
            ConfigValue::Boolean(b) => ConfigValueResponse::Boolean(*b),
            ConfigValue::StringList(list) => ConfigValueResponse::StringList(list.clone()),
        }
    }
}

/// List configuration response
#[derive(Debug, Clone, Serialize)]
pub struct ListConfigResponse {
    pub config: Vec<ConfigEntryResponse>,
}

/// Request to update a configuration value
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateConfigRequest {
    pub value: ConfigValueRequest,
}

/// Configuration value in request format
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum ConfigValueRequest {
    #[serde(rename = "string")]
    String(String),
    #[serde(rename = "integer")]
    Integer(i64),
    #[serde(rename = "float")]
    Float(f64),
    #[serde(rename = "boolean")]
    Boolean(bool),
    #[serde(rename = "string_list")]
    StringList(Vec<String>),
}

impl From<ConfigValueRequest> for ConfigValue {
    fn from(req: ConfigValueRequest) -> Self {
        match req {
            ConfigValueRequest::String(s) => ConfigValue::String(s),
            ConfigValueRequest::Integer(i) => ConfigValue::Integer(i),
            ConfigValueRequest::Float(f) => ConfigValue::Float(f),
            ConfigValueRequest::Boolean(b) => ConfigValue::Boolean(b),
            ConfigValueRequest::StringList(list) => ConfigValue::StringList(list),
        }
    }
}

/// List all configuration entries
pub async fn list_config(
    _admin: RequireAdmin,
    State(state): State<AppState>,
) -> Result<Json<ListConfigResponse>, ApiError> {
    let entries = state.config_service.list().await?;

    let config = entries
        .into_iter()
        .map(|entry| ConfigEntryResponse {
            key: entry.key().to_string(),
            value: ConfigValueResponse::from(entry.value()),
            category: entry.category().to_string(),
            description: entry.description().map(|s| s.to_string()),
            updated_at: entry.updated_at().to_rfc3339(),
        })
        .collect();

    Ok(Json(ListConfigResponse { config }))
}

/// List configuration entries by category
pub async fn list_config_by_category(
    _admin: RequireAdmin,
    State(state): State<AppState>,
    Path(category): Path<String>,
) -> Result<Json<ListConfigResponse>, ApiError> {
    let category = match category.to_lowercase().as_str() {
        "general" => ConfigCategory::General,
        "persistence" => ConfigCategory::Persistence,
        "logging" => ConfigCategory::Logging,
        "security" => ConfigCategory::Security,
        "cache" => ConfigCategory::Cache,
        "rate_limit" => ConfigCategory::RateLimit,
        _ => {
            return Err(ApiError::bad_request(format!(
                "Unknown category: {}",
                category
            )))
        }
    };

    let entries = state.config_service.list_by_category(category).await?;

    let config = entries
        .into_iter()
        .map(|entry| ConfigEntryResponse {
            key: entry.key().to_string(),
            value: ConfigValueResponse::from(entry.value()),
            category: entry.category().to_string(),
            description: entry.description().map(|s| s.to_string()),
            updated_at: entry.updated_at().to_rfc3339(),
        })
        .collect();

    Ok(Json(ListConfigResponse { config }))
}

/// Get a specific configuration entry
pub async fn get_config(
    _admin: RequireAdmin,
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<ConfigEntryResponse>, ApiError> {
    let entry = state
        .config_service
        .get_entry(&key)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("Configuration key '{}' not found", key)))?;

    Ok(Json(ConfigEntryResponse {
        key: entry.key().to_string(),
        value: ConfigValueResponse::from(entry.value()),
        category: entry.category().to_string(),
        description: entry.description().map(|s| s.to_string()),
        updated_at: entry.updated_at().to_rfc3339(),
    }))
}

/// Update a configuration value
pub async fn update_config(
    _admin: RequireAdmin,
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(request): Json<UpdateConfigRequest>,
) -> Result<Json<ConfigEntryResponse>, ApiError> {
    let value: ConfigValue = request.value.into();
    state.config_service.set(&key, value).await?;

    let entry = state
        .config_service
        .get_entry(&key)
        .await?
        .ok_or_else(|| ApiError::internal("Failed to retrieve updated config"))?;

    Ok(Json(ConfigEntryResponse {
        key: entry.key().to_string(),
        value: ConfigValueResponse::from(entry.value()),
        category: entry.category().to_string(),
        description: entry.description().map(|s| s.to_string()),
        updated_at: entry.updated_at().to_rfc3339(),
    }))
}

/// Reset configuration to defaults
pub async fn reset_config(
    _admin: RequireAdmin,
    State(state): State<AppState>,
) -> Result<Json<ListConfigResponse>, ApiError> {
    state.config_service.reset().await?;

    let entries = state.config_service.list().await?;

    let config = entries
        .into_iter()
        .map(|entry| ConfigEntryResponse {
            key: entry.key().to_string(),
            value: ConfigValueResponse::from(entry.value()),
            category: entry.category().to_string(),
            description: entry.description().map(|s| s.to_string()),
            updated_at: entry.updated_at().to_rfc3339(),
        })
        .collect();

    Ok(Json(ListConfigResponse { config }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_value_response_string_serialization() {
        let value = ConfigValueResponse::String("test".to_string());
        let json = serde_json::to_string(&value).unwrap();
        assert!(json.contains("\"type\":\"string\""));
        assert!(json.contains("\"value\":\"test\""));
    }

    #[test]
    fn test_config_value_response_integer_serialization() {
        let value = ConfigValueResponse::Integer(42);
        let json = serde_json::to_string(&value).unwrap();
        assert!(json.contains("\"type\":\"integer\""));
        assert!(json.contains("\"value\":42"));
    }

    #[test]
    fn test_config_value_response_float_serialization() {
        let value = ConfigValueResponse::Float(3.14);
        let json = serde_json::to_string(&value).unwrap();
        assert!(json.contains("\"type\":\"float\""));
        assert!(json.contains("3.14"));
    }

    #[test]
    fn test_config_value_response_boolean_serialization() {
        let value = ConfigValueResponse::Boolean(true);
        let json = serde_json::to_string(&value).unwrap();
        assert!(json.contains("\"type\":\"boolean\""));
        assert!(json.contains("\"value\":true"));
    }

    #[test]
    fn test_config_value_response_string_list_serialization() {
        let value = ConfigValueResponse::StringList(vec!["a".to_string(), "b".to_string()]);
        let json = serde_json::to_string(&value).unwrap();
        assert!(json.contains("\"type\":\"string_list\""));
        assert!(json.contains("[\"a\",\"b\"]"));
    }

    #[test]
    fn test_config_value_response_from_string() {
        let domain = ConfigValue::String("hello".to_string());
        let response = ConfigValueResponse::from(&domain);

        if let ConfigValueResponse::String(s) = response {
            assert_eq!(s, "hello");
        } else {
            panic!("Expected String variant");
        }
    }

    #[test]
    fn test_config_value_response_from_integer() {
        let domain = ConfigValue::Integer(100);
        let response = ConfigValueResponse::from(&domain);

        if let ConfigValueResponse::Integer(i) = response {
            assert_eq!(i, 100);
        } else {
            panic!("Expected Integer variant");
        }
    }

    #[test]
    fn test_config_value_response_from_float() {
        let domain = ConfigValue::Float(2.5);
        let response = ConfigValueResponse::from(&domain);

        if let ConfigValueResponse::Float(f) = response {
            assert!((f - 2.5).abs() < f64::EPSILON);
        } else {
            panic!("Expected Float variant");
        }
    }

    #[test]
    fn test_config_value_response_from_boolean() {
        let domain = ConfigValue::Boolean(false);
        let response = ConfigValueResponse::from(&domain);

        if let ConfigValueResponse::Boolean(b) = response {
            assert!(!b);
        } else {
            panic!("Expected Boolean variant");
        }
    }

    #[test]
    fn test_config_value_response_from_string_list() {
        let domain = ConfigValue::StringList(vec!["x".to_string(), "y".to_string()]);
        let response = ConfigValueResponse::from(&domain);

        if let ConfigValueResponse::StringList(list) = response {
            assert_eq!(list, vec!["x", "y"]);
        } else {
            panic!("Expected StringList variant");
        }
    }

    #[test]
    fn test_config_entry_response_serialization() {
        let entry = ConfigEntryResponse {
            key: "app.name".to_string(),
            value: ConfigValueResponse::String("MyApp".to_string()),
            category: "general".to_string(),
            description: Some("Application name".to_string()),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"key\":\"app.name\""));
        assert!(json.contains("\"category\":\"general\""));
        assert!(json.contains("\"description\":\"Application name\""));
    }

    #[test]
    fn test_config_entry_response_without_description() {
        let entry = ConfigEntryResponse {
            key: "app.setting".to_string(),
            value: ConfigValueResponse::Integer(10),
            category: "cache".to_string(),
            description: None,
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"key\":\"app.setting\""));
        assert!(json.contains("\"description\":null"));
    }

    #[test]
    fn test_list_config_response_serialization() {
        let response = ListConfigResponse {
            config: vec![
                ConfigEntryResponse {
                    key: "key1".to_string(),
                    value: ConfigValueResponse::String("value1".to_string()),
                    category: "general".to_string(),
                    description: None,
                    updated_at: "2024-01-01T00:00:00Z".to_string(),
                },
            ],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"config\":"));
        assert!(json.contains("\"key1\""));
    }

    #[test]
    fn test_config_value_request_string_deserialization() {
        let json = r#"{"type":"string","value":"test"}"#;
        let value: ConfigValueRequest = serde_json::from_str(json).unwrap();

        if let ConfigValueRequest::String(s) = value {
            assert_eq!(s, "test");
        } else {
            panic!("Expected String variant");
        }
    }

    #[test]
    fn test_config_value_request_integer_deserialization() {
        let json = r#"{"type":"integer","value":42}"#;
        let value: ConfigValueRequest = serde_json::from_str(json).unwrap();

        if let ConfigValueRequest::Integer(i) = value {
            assert_eq!(i, 42);
        } else {
            panic!("Expected Integer variant");
        }
    }

    #[test]
    fn test_config_value_request_float_deserialization() {
        let json = r#"{"type":"float","value":3.14}"#;
        let value: ConfigValueRequest = serde_json::from_str(json).unwrap();

        if let ConfigValueRequest::Float(f) = value {
            assert!((f - 3.14).abs() < 0.001);
        } else {
            panic!("Expected Float variant");
        }
    }

    #[test]
    fn test_config_value_request_boolean_deserialization() {
        let json = r#"{"type":"boolean","value":true}"#;
        let value: ConfigValueRequest = serde_json::from_str(json).unwrap();

        if let ConfigValueRequest::Boolean(b) = value {
            assert!(b);
        } else {
            panic!("Expected Boolean variant");
        }
    }

    #[test]
    fn test_config_value_request_string_list_deserialization() {
        let json = r#"{"type":"string_list","value":["a","b","c"]}"#;
        let value: ConfigValueRequest = serde_json::from_str(json).unwrap();

        if let ConfigValueRequest::StringList(list) = value {
            assert_eq!(list, vec!["a", "b", "c"]);
        } else {
            panic!("Expected StringList variant");
        }
    }

    #[test]
    fn test_config_value_request_to_domain_string() {
        let request = ConfigValueRequest::String("hello".to_string());
        let domain: ConfigValue = request.into();

        if let ConfigValue::String(s) = domain {
            assert_eq!(s, "hello");
        } else {
            panic!("Expected String variant");
        }
    }

    #[test]
    fn test_config_value_request_to_domain_integer() {
        let request = ConfigValueRequest::Integer(123);
        let domain: ConfigValue = request.into();

        if let ConfigValue::Integer(i) = domain {
            assert_eq!(i, 123);
        } else {
            panic!("Expected Integer variant");
        }
    }

    #[test]
    fn test_config_value_request_to_domain_float() {
        let request = ConfigValueRequest::Float(1.5);
        let domain: ConfigValue = request.into();

        if let ConfigValue::Float(f) = domain {
            assert!((f - 1.5).abs() < f64::EPSILON);
        } else {
            panic!("Expected Float variant");
        }
    }

    #[test]
    fn test_config_value_request_to_domain_boolean() {
        let request = ConfigValueRequest::Boolean(false);
        let domain: ConfigValue = request.into();

        if let ConfigValue::Boolean(b) = domain {
            assert!(!b);
        } else {
            panic!("Expected Boolean variant");
        }
    }

    #[test]
    fn test_config_value_request_to_domain_string_list() {
        let request = ConfigValueRequest::StringList(vec!["x".to_string()]);
        let domain: ConfigValue = request.into();

        if let ConfigValue::StringList(list) = domain {
            assert_eq!(list, vec!["x"]);
        } else {
            panic!("Expected StringList variant");
        }
    }

    #[test]
    fn test_update_config_request_deserialization() {
        let json = r#"{"value":{"type":"string","value":"new_value"}}"#;
        let request: UpdateConfigRequest = serde_json::from_str(json).unwrap();

        if let ConfigValueRequest::String(s) = request.value {
            assert_eq!(s, "new_value");
        } else {
            panic!("Expected String variant");
        }
    }

    #[test]
    fn test_update_config_request_with_integer() {
        let json = r#"{"value":{"type":"integer","value":999}}"#;
        let request: UpdateConfigRequest = serde_json::from_str(json).unwrap();

        if let ConfigValueRequest::Integer(i) = request.value {
            assert_eq!(i, 999);
        } else {
            panic!("Expected Integer variant");
        }
    }
}
