//! Credentials management admin endpoints

use axum::extract::{Path, State};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use tracing::debug;

use crate::api::middleware::RequireAdmin;
use crate::api::state::AppState;
use crate::api::types::{ApiError, Json};
use crate::domain::credentials::{Credential, CredentialType, StoredCredential};
use crate::domain::llm::{LlmRequest, Message};
use crate::infrastructure::llm::{LlmProviderConfig, LlmProviderFactory};

/// Credential provider info response
#[derive(Debug, Clone, Serialize)]
pub struct CredentialProviderInfo {
    pub provider_type: String,
    pub description: String,
}

/// List credentials response
#[derive(Debug, Clone, Serialize)]
pub struct ListCredentialProvidersResponse {
    pub providers: Vec<CredentialProviderInfo>,
}

/// Request to create a new stored credential
#[derive(Debug, Clone, Deserialize)]
pub struct CreateCredentialApiRequest {
    pub id: String,
    pub name: String,
    pub credential_type: String,
    /// API key - required for OpenAI, Anthropic, Azure, Pinecone, HTTP API Key; optional for AWS/pgvector
    #[serde(default)]
    pub api_key: Option<String>,
    /// Endpoint URL - for Azure OpenAI, AWS region, or pgvector connection string
    pub endpoint: Option<String>,
    /// Deployment (Azure OpenAI) or header name (HTTP API Key, e.g., "Authorization")
    pub deployment: Option<String>,
    /// Header value template for HTTP API Key credentials (e.g., "Bearer ${api-key}")
    pub header_value: Option<String>,
}

/// Request to update a stored credential
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateCredentialApiRequest {
    pub name: Option<String>,
    pub api_key: Option<String>,
    pub endpoint: Option<Option<String>>,
    pub deployment: Option<Option<String>>,
    pub header_value: Option<Option<String>>,
    pub enabled: Option<bool>,
}

/// Stored credential response
#[derive(Debug, Clone, Serialize)]
pub struct CredentialResponse {
    pub id: String,
    pub name: String,
    pub credential_type: String,
    pub endpoint: Option<String>,
    pub deployment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header_value: Option<String>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// List credentials response
#[derive(Debug, Clone, Serialize)]
pub struct ListCredentialsResponse {
    pub credentials: Vec<CredentialResponse>,
    pub total: usize,
}

fn credential_type_to_string(ct: &CredentialType) -> String {
    match ct {
        CredentialType::OpenAi => "openai".to_string(),
        CredentialType::Anthropic => "anthropic".to_string(),
        CredentialType::AzureOpenAi => "azure_openai".to_string(),
        CredentialType::AwsBedrock => "aws_bedrock".to_string(),
        CredentialType::Pgvector => "pgvector".to_string(),
        CredentialType::AwsKnowledgeBase => "aws_knowledge_base".to_string(),
        CredentialType::Pinecone => "pinecone".to_string(),
        CredentialType::HttpApiKey => "http_api_key".to_string(),
        CredentialType::Custom(s) => s.clone(),
    }
}

fn parse_credential_type(s: &str) -> Result<CredentialType, ApiError> {
    match s.to_lowercase().as_str() {
        "openai" => Ok(CredentialType::OpenAi),
        "anthropic" => Ok(CredentialType::Anthropic),
        "azure_openai" | "azure-openai" | "azureopenai" => Ok(CredentialType::AzureOpenAi),
        "aws_bedrock" | "aws-bedrock" | "awsbedrock" | "bedrock" => Ok(CredentialType::AwsBedrock),
        "pgvector" | "pg_vector" => Ok(CredentialType::Pgvector),
        "aws_knowledge_base" | "aws-knowledge-base" | "awsknowledgebase" => {
            Ok(CredentialType::AwsKnowledgeBase)
        }
        "pinecone" => Ok(CredentialType::Pinecone),
        "http_api_key" | "http-api-key" | "httpapikey" => Ok(CredentialType::HttpApiKey),
        other => Ok(CredentialType::Custom(other.to_string())),
    }
}

impl From<&StoredCredential> for CredentialResponse {
    fn from(cred: &StoredCredential) -> Self {
        // Hide connection string for Pgvector credentials (sensitive data)
        let endpoint = if matches!(cred.credential_type(), CredentialType::Pgvector) {
            cred.endpoint().map(|_| "[HIDDEN]".to_string())
        } else {
            cred.endpoint().map(|s| s.to_string())
        };

        Self {
            id: cred.id().as_str().to_string(),
            name: cred.name().to_string(),
            credential_type: credential_type_to_string(cred.credential_type()),
            endpoint,
            deployment: cred.deployment().map(|s| s.to_string()),
            header_value: cred.header_value().map(|s| s.to_string()),
            enabled: cred.is_enabled(),
            created_at: cred.created_at().to_rfc3339(),
            updated_at: cred.updated_at().to_rfc3339(),
        }
    }
}

/// GET /admin/credentials/providers
/// Lists available credential provider types
pub async fn list_credential_providers(
    State(_state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
) -> Result<Json<ListCredentialProvidersResponse>, ApiError> {
    debug!("Admin listing credential providers");

    let providers = vec![
        // LLM Providers
        CredentialProviderInfo {
            provider_type: credential_type_to_string(&CredentialType::OpenAi),
            description: "OpenAI API credentials".to_string(),
        },
        CredentialProviderInfo {
            provider_type: credential_type_to_string(&CredentialType::Anthropic),
            description: "Anthropic API credentials".to_string(),
        },
        CredentialProviderInfo {
            provider_type: credential_type_to_string(&CredentialType::AzureOpenAi),
            description: "Azure OpenAI API credentials".to_string(),
        },
        CredentialProviderInfo {
            provider_type: credential_type_to_string(&CredentialType::AwsBedrock),
            description: "AWS Bedrock credentials".to_string(),
        },
        // Knowledge Base Providers
        CredentialProviderInfo {
            provider_type: credential_type_to_string(&CredentialType::Pgvector),
            description: "PostgreSQL pgvector credentials".to_string(),
        },
        CredentialProviderInfo {
            provider_type: credential_type_to_string(&CredentialType::AwsKnowledgeBase),
            description: "AWS Bedrock Knowledge Base credentials".to_string(),
        },
        CredentialProviderInfo {
            provider_type: credential_type_to_string(&CredentialType::Pinecone),
            description: "Pinecone vector database credentials".to_string(),
        },
    ];

    Ok(Json(ListCredentialProvidersResponse { providers }))
}

/// GET /admin/credentials
/// List all stored credentials
pub async fn list_credentials(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
) -> Result<Json<ListCredentialsResponse>, ApiError> {
    debug!("Admin listing all credentials");

    let credentials = state
        .credential_service
        .list()
        .await
        .map_err(ApiError::from)?;

    let cred_responses: Vec<CredentialResponse> =
        credentials.iter().map(CredentialResponse::from).collect();
    let total = cred_responses.len();

    Ok(Json(ListCredentialsResponse {
        credentials: cred_responses,
        total,
    }))
}

/// Check if credential type requires an API key
fn requires_api_key(credential_type: &CredentialType) -> bool {
    matches!(
        credential_type,
        CredentialType::OpenAi
            | CredentialType::Anthropic
            | CredentialType::AzureOpenAi
            | CredentialType::Pinecone
            | CredentialType::HttpApiKey
    )
}

/// POST /admin/credentials
/// Create a new stored credential
pub async fn create_credential(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Json(request): Json<CreateCredentialApiRequest>,
) -> Result<Json<CredentialResponse>, ApiError> {
    debug!(credential_id = %request.id, "Admin creating credential");

    let credential_type = parse_credential_type(&request.credential_type)?;

    // Validate API key is present for providers that require it
    let api_key = if requires_api_key(&credential_type) {
        request.api_key.filter(|k| !k.is_empty()).ok_or_else(|| {
            ApiError::bad_request(format!(
                "API key is required for {} credentials",
                request.credential_type
            ))
        })?
    } else {
        // For AWS and pgvector, use empty string if not provided
        request.api_key.unwrap_or_default()
    };

    let create_request = crate::infrastructure::credentials::CreateCredentialRequest {
        id: request.id,
        name: request.name,
        credential_type,
        api_key,
        endpoint: request.endpoint,
        deployment: request.deployment,
        header_value: request.header_value,
    };

    let credential = state
        .credential_service
        .create(create_request)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(CredentialResponse::from(&credential)))
}

/// GET /admin/credentials/:credential_id
/// Get a specific stored credential
pub async fn get_credential(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(credential_id): Path<String>,
) -> Result<Json<CredentialResponse>, ApiError> {
    debug!(credential_id = %credential_id, "Admin getting credential");

    let credential = state
        .credential_service
        .get(&credential_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::not_found(format!("Credential '{}' not found", credential_id)))?;

    Ok(Json(CredentialResponse::from(&credential)))
}

/// PUT /admin/credentials/:credential_id
/// Update a stored credential
pub async fn update_credential(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(credential_id): Path<String>,
    Json(request): Json<UpdateCredentialApiRequest>,
) -> Result<Json<CredentialResponse>, ApiError> {
    debug!(credential_id = %credential_id, "Admin updating credential");

    let update_request = crate::infrastructure::credentials::UpdateCredentialRequest {
        name: request.name,
        api_key: request.api_key,
        endpoint: request.endpoint,
        deployment: request.deployment,
        header_value: request.header_value,
        enabled: request.enabled,
    };

    let credential = state
        .credential_service
        .update(&credential_id, update_request)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(CredentialResponse::from(&credential)))
}

/// DELETE /admin/credentials/:credential_id
/// Delete a stored credential
pub async fn delete_credential(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(credential_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    debug!(credential_id = %credential_id, "Admin deleting credential");

    // Check if any models are using this credential
    let models = state.model_service.list().await.map_err(ApiError::from)?;
    let models_using_credential: Vec<_> = models
        .iter()
        .filter(|m| m.credential_id() == credential_id)
        .map(|m| m.id().as_str().to_string())
        .collect();

    if !models_using_credential.is_empty() {
        return Err(ApiError::conflict(format!(
            "Cannot delete credential '{}': it is used by {} model(s): {}",
            credential_id,
            models_using_credential.len(),
            models_using_credential.join(", ")
        )));
    }

    state
        .credential_service
        .delete(&credential_id)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(serde_json::json!({
        "deleted": true,
        "id": credential_id
    })))
}

/// Request to test a credential
#[derive(Debug, Clone, Deserialize)]
pub struct TestCredentialRequest {
    pub message: String,
    #[serde(default = "default_test_model")]
    pub model: String,
}

fn default_test_model() -> String {
    "gpt-4o-mini".to_string()
}

/// Response from testing a credential
#[derive(Debug, Clone, Serialize)]
pub struct TestCredentialResponse {
    pub success: bool,
    pub provider: String,
    pub model: String,
    pub response: Option<String>,
    pub error: Option<String>,
    pub latency_ms: u64,
}

/// POST /admin/credentials/:credential_id/test
/// Test a credential by sending a simple chat request or testing database connection
pub async fn test_credential(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(credential_id): Path<String>,
    Json(request): Json<TestCredentialRequest>,
) -> Result<Json<TestCredentialResponse>, ApiError> {
    debug!(credential_id = %credential_id, "Admin testing credential");

    let start = Instant::now();

    let stored_cred = state
        .credential_service
        .get(&credential_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::not_found(format!("Credential '{}' not found", credential_id)))?;

    let provider_name = credential_type_to_string(stored_cred.credential_type());

    if !stored_cred.is_enabled() {
        return Ok(Json(TestCredentialResponse {
            success: false,
            provider: provider_name,
            model: request.model,
            response: None,
            error: Some("Credential is disabled".to_string()),
            latency_ms: start.elapsed().as_millis() as u64,
        }));
    }

    // Handle Pgvector credentials - test database connection
    if matches!(stored_cred.credential_type(), CredentialType::Pgvector) {
        return test_pgvector_credential(&stored_cred, start).await;
    }

    let credential = Credential::new(
        stored_cred.credential_type().clone(),
        stored_cred.api_key().to_string(),
    );

    let provider_config = create_provider_config(&stored_cred)?;

    let provider = match stored_cred.credential_type() {
        CredentialType::AwsBedrock => {
            let region = stored_cred.endpoint();
            match LlmProviderFactory::create_bedrock_async(region).await {
                Ok(p) => p,
                Err(e) => {
                    return Ok(Json(TestCredentialResponse {
                        success: false,
                        provider: provider_name,
                        model: request.model,
                        response: None,
                        error: Some(format!("Failed to create provider: {}", e)),
                        latency_ms: start.elapsed().as_millis() as u64,
                    }));
                }
            }
        }
        _ => match LlmProviderFactory::create(&provider_config, &credential) {
            Ok(p) => p,
            Err(e) => {
                return Ok(Json(TestCredentialResponse {
                    success: false,
                    provider: provider_name,
                    model: request.model,
                    response: None,
                    error: Some(format!("Failed to create provider: {}", e)),
                    latency_ms: start.elapsed().as_millis() as u64,
                }));
            }
        },
    };

    let llm_request = LlmRequest::new(vec![Message::user(request.message)]);

    match provider.chat(&request.model, llm_request).await {
        Ok(response) => Ok(Json(TestCredentialResponse {
            success: true,
            provider: provider_name,
            model: request.model,
            response: response.content().map(|s| s.to_string()),
            error: None,
            latency_ms: start.elapsed().as_millis() as u64,
        })),
        Err(e) => Ok(Json(TestCredentialResponse {
            success: false,
            provider: provider_name,
            model: request.model,
            response: None,
            error: Some(e.to_string()),
            latency_ms: start.elapsed().as_millis() as u64,
        })),
    }
}

/// Test a pgvector credential by connecting to the database
async fn test_pgvector_credential(
    cred: &StoredCredential,
    start: Instant,
) -> Result<Json<TestCredentialResponse>, ApiError> {
    let connection_string = cred.api_key();

    // Try to connect to the database
    match sqlx::PgPool::connect(connection_string).await {
        Ok(pool) => {
            // Run a simple query to verify the connection works
            match sqlx::query("SELECT 1 as test").fetch_one(&pool).await {
                Ok(_) => {
                    // Check if pgvector extension is available
                    let pgvector_check = sqlx::query(
                        "SELECT EXISTS(SELECT 1 FROM pg_extension WHERE extname = 'vector') as has_pgvector"
                    )
                    .fetch_one(&pool)
                    .await;

                    let response_msg = match pgvector_check {
                        Ok(row) => {
                            use sqlx::Row;
                            let has_pgvector: bool = row.get("has_pgvector");

                            if has_pgvector {
                                "Connection successful. pgvector extension is installed."
                            } else {
                                "Connection successful. Warning: pgvector extension is NOT installed."
                            }
                        }
                        Err(_) => "Connection successful. Could not verify pgvector extension.",
                    };

                    Ok(Json(TestCredentialResponse {
                        success: true,
                        provider: "pgvector".to_string(),
                        model: "PostgreSQL".to_string(),
                        response: Some(response_msg.to_string()),
                        error: None,
                        latency_ms: start.elapsed().as_millis() as u64,
                    }))
                }
                Err(e) => Ok(Json(TestCredentialResponse {
                    success: false,
                    provider: "pgvector".to_string(),
                    model: "PostgreSQL".to_string(),
                    response: None,
                    error: Some(format!("Connected but query failed: {}", e)),
                    latency_ms: start.elapsed().as_millis() as u64,
                })),
            }
        }
        Err(e) => Ok(Json(TestCredentialResponse {
            success: false,
            provider: "pgvector".to_string(),
            model: "PostgreSQL".to_string(),
            response: None,
            error: Some(format!("Failed to connect: {}", e)),
            latency_ms: start.elapsed().as_millis() as u64,
        })),
    }
}

fn create_provider_config(cred: &StoredCredential) -> Result<LlmProviderConfig, ApiError> {
    match cred.credential_type() {
        CredentialType::OpenAi => Ok(LlmProviderConfig::OpenAi),
        CredentialType::Anthropic => Ok(LlmProviderConfig::Anthropic),
        CredentialType::AzureOpenAi => {
            let endpoint = cred
                .endpoint()
                .ok_or_else(|| ApiError::bad_request("Azure OpenAI requires an endpoint"))?;
            Ok(LlmProviderConfig::AzureOpenAi {
                endpoint: endpoint.to_string(),
                api_version: "2024-02-01".to_string(),
            })
        }
        CredentialType::AwsBedrock => Ok(LlmProviderConfig::AwsBedrock {
            region: cred.endpoint().map(|s| s.to_string()),
        }),
        CredentialType::Pgvector
        | CredentialType::AwsKnowledgeBase
        | CredentialType::Pinecone => Err(ApiError::bad_request(
            "Knowledge Base credentials cannot be tested as LLM providers",
        )),
        CredentialType::HttpApiKey => Err(ApiError::bad_request(
            "HTTP API Key credentials cannot be tested as LLM providers",
        )),
        CredentialType::Custom(name) => {
            Err(ApiError::bad_request(format!("Unsupported provider: {}", name)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credential_type_to_string() {
        assert_eq!(credential_type_to_string(&CredentialType::OpenAi), "openai");
        assert_eq!(credential_type_to_string(&CredentialType::Anthropic), "anthropic");
        assert_eq!(credential_type_to_string(&CredentialType::AzureOpenAi), "azure_openai");
        assert_eq!(credential_type_to_string(&CredentialType::AwsBedrock), "aws_bedrock");
        assert_eq!(credential_type_to_string(&CredentialType::Pgvector), "pgvector");
        assert_eq!(credential_type_to_string(&CredentialType::AwsKnowledgeBase), "aws_knowledge_base");
        assert_eq!(credential_type_to_string(&CredentialType::Pinecone), "pinecone");
        assert_eq!(credential_type_to_string(&CredentialType::HttpApiKey), "http_api_key");
        assert_eq!(credential_type_to_string(&CredentialType::Custom("custom".to_string())), "custom");
    }

    #[test]
    fn test_parse_credential_type() {
        assert!(matches!(parse_credential_type("openai").unwrap(), CredentialType::OpenAi));
        assert!(matches!(parse_credential_type("anthropic").unwrap(), CredentialType::Anthropic));
        assert!(matches!(parse_credential_type("azure_openai").unwrap(), CredentialType::AzureOpenAi));
    }

    #[test]
    fn test_parse_credential_type_variants() {
        assert!(matches!(parse_credential_type("azure-openai").unwrap(), CredentialType::AzureOpenAi));
        assert!(matches!(parse_credential_type("azureopenai").unwrap(), CredentialType::AzureOpenAi));
        assert!(matches!(parse_credential_type("aws_bedrock").unwrap(), CredentialType::AwsBedrock));
        assert!(matches!(parse_credential_type("aws-bedrock").unwrap(), CredentialType::AwsBedrock));
        assert!(matches!(parse_credential_type("bedrock").unwrap(), CredentialType::AwsBedrock));
        assert!(matches!(parse_credential_type("pgvector").unwrap(), CredentialType::Pgvector));
        assert!(matches!(parse_credential_type("pg_vector").unwrap(), CredentialType::Pgvector));
        assert!(matches!(parse_credential_type("pinecone").unwrap(), CredentialType::Pinecone));
        assert!(matches!(parse_credential_type("http_api_key").unwrap(), CredentialType::HttpApiKey));
        assert!(matches!(parse_credential_type("http-api-key").unwrap(), CredentialType::HttpApiKey));
    }

    #[test]
    fn test_parse_credential_type_custom() {
        let result = parse_credential_type("my-custom-provider").unwrap();
        assert!(matches!(result, CredentialType::Custom(s) if s == "my-custom-provider"));
    }

    #[test]
    fn test_requires_api_key() {
        assert!(requires_api_key(&CredentialType::OpenAi));
        assert!(requires_api_key(&CredentialType::Anthropic));
        assert!(requires_api_key(&CredentialType::AzureOpenAi));
        assert!(requires_api_key(&CredentialType::Pinecone));
        assert!(requires_api_key(&CredentialType::HttpApiKey));
        assert!(!requires_api_key(&CredentialType::AwsBedrock));
        assert!(!requires_api_key(&CredentialType::Pgvector));
        assert!(!requires_api_key(&CredentialType::AwsKnowledgeBase));
    }

    #[test]
    fn test_default_test_model() {
        assert_eq!(default_test_model(), "gpt-4o-mini");
    }

    #[test]
    fn test_create_credential_request_deserialization_full() {
        let json = r#"{
            "id": "my-cred",
            "name": "My Credential",
            "credential_type": "openai",
            "api_key": "sk-test-key",
            "endpoint": "https://api.example.com",
            "deployment": "gpt-4"
        }"#;

        let request: CreateCredentialApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.id, "my-cred");
        assert_eq!(request.name, "My Credential");
        assert_eq!(request.credential_type, "openai");
        assert_eq!(request.api_key, Some("sk-test-key".to_string()));
        assert_eq!(request.endpoint, Some("https://api.example.com".to_string()));
        assert_eq!(request.deployment, Some("gpt-4".to_string()));
    }

    #[test]
    fn test_create_credential_request_minimal() {
        let json = r#"{
            "id": "test-cred",
            "name": "Test",
            "credential_type": "aws_bedrock"
        }"#;

        let request: CreateCredentialApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.id, "test-cred");
        assert!(request.api_key.is_none());
        assert!(request.endpoint.is_none());
        assert!(request.deployment.is_none());
        assert!(request.header_value.is_none());
    }

    #[test]
    fn test_create_credential_request_http_api_key() {
        let json = r#"{
            "id": "ext-api",
            "name": "External API",
            "credential_type": "http_api_key",
            "api_key": "my-secret-key",
            "deployment": "Authorization",
            "header_value": "Bearer ${api-key}"
        }"#;

        let request: CreateCredentialApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.credential_type, "http_api_key");
        assert_eq!(request.deployment, Some("Authorization".to_string()));
        assert_eq!(request.header_value, Some("Bearer ${api-key}".to_string()));
    }

    #[test]
    fn test_update_credential_request_partial() {
        let json = r#"{"name": "New Name"}"#;

        let request: UpdateCredentialApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.name, Some("New Name".to_string()));
        assert!(request.api_key.is_none());
        assert!(request.endpoint.is_none());
        assert!(request.deployment.is_none());
        assert!(request.enabled.is_none());
    }

    #[test]
    fn test_update_credential_request_full() {
        let json = r#"{
            "name": "Updated",
            "api_key": "new-key",
            "endpoint": "https://new.api.com",
            "deployment": "new-deployment",
            "enabled": false
        }"#;

        let request: UpdateCredentialApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.name, Some("Updated".to_string()));
        assert_eq!(request.api_key, Some("new-key".to_string()));
        assert_eq!(request.endpoint, Some(Some("https://new.api.com".to_string())));
        assert_eq!(request.enabled, Some(false));
    }

    #[test]
    fn test_test_credential_request_deserialization() {
        let json = r#"{"message": "Hello"}"#;

        let request: TestCredentialRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.message, "Hello");
        assert_eq!(request.model, "gpt-4o-mini");
    }

    #[test]
    fn test_test_credential_request_with_model() {
        let json = r#"{"message": "Hello", "model": "gpt-4"}"#;

        let request: TestCredentialRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.message, "Hello");
        assert_eq!(request.model, "gpt-4");
    }

    #[test]
    fn test_credential_response_serialization() {
        let response = CredentialResponse {
            id: "test-cred".to_string(),
            name: "Test Credential".to_string(),
            credential_type: "openai".to_string(),
            endpoint: None,
            deployment: None,
            header_value: None,
            enabled: true,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"id\":\"test-cred\""));
        assert!(json.contains("\"credential_type\":\"openai\""));
        assert!(json.contains("\"enabled\":true"));
        assert!(!json.contains("header_value"));
    }

    #[test]
    fn test_test_credential_response_success() {
        let response = TestCredentialResponse {
            success: true,
            provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            response: Some("Hello!".to_string()),
            error: None,
            latency_ms: 150,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"response\":\"Hello!\""));
        assert!(json.contains("\"latency_ms\":150"));
    }

    #[test]
    fn test_test_credential_response_failure() {
        let response = TestCredentialResponse {
            success: false,
            provider: "anthropic".to_string(),
            model: "claude-3".to_string(),
            response: None,
            error: Some("Connection failed".to_string()),
            latency_ms: 50,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":false"));
        assert!(json.contains("\"error\":\"Connection failed\""));
    }

    #[test]
    fn test_list_credentials_response_serialization() {
        let response = ListCredentialsResponse {
            credentials: vec![],
            total: 0,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"credentials\":[]"));
        assert!(json.contains("\"total\":0"));
    }

    #[test]
    fn test_credential_provider_info_serialization() {
        let info = CredentialProviderInfo {
            provider_type: "openai".to_string(),
            description: "OpenAI API credentials".to_string(),
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"provider_type\":\"openai\""));
        assert!(json.contains("\"description\":\"OpenAI API credentials\""));
    }
}
