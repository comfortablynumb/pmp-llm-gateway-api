//! OpenAI-compatible model types

use serde::{Deserialize, Serialize};

/// Model information (OpenAI format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub owned_by: String,
}

impl Model {
    /// Create a model from domain model
    pub fn from_domain(model: &crate::domain::model::Model) -> Self {
        Self {
            id: model.id().as_str().to_string(),
            object: "model".to_string(),
            created: model.created_at().timestamp(),
            owned_by: model.provider().to_string(),
        }
    }
}

/// List models response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsResponse {
    pub object: String,
    pub data: Vec<Model>,
}

impl ModelsResponse {
    /// Create a new models response
    pub fn new(models: Vec<Model>) -> Self {
        Self {
            object: "list".to_string(),
            data: models,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_serialization() {
        let model = Model {
            id: "gpt-4".to_string(),
            object: "model".to_string(),
            created: 1234567890,
            owned_by: "openai".to_string(),
        };

        let json = serde_json::to_string(&model).unwrap();
        assert!(json.contains("gpt-4"));
        assert!(json.contains("openai"));
    }

    #[test]
    fn test_models_response() {
        let models = vec![
            Model {
                id: "model-1".to_string(),
                object: "model".to_string(),
                created: 1234567890,
                owned_by: "owner".to_string(),
            },
            Model {
                id: "model-2".to_string(),
                object: "model".to_string(),
                created: 1234567890,
                owned_by: "owner".to_string(),
            },
        ];

        let response = ModelsResponse::new(models);
        assert_eq!(response.object, "list");
        assert_eq!(response.data.len(), 2);
    }
}
