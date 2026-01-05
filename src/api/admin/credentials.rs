//! Credentials info admin endpoints

use axum::{extract::State, Json};
use serde::Serialize;
use tracing::debug;

use crate::api::middleware::RequireApiKey;
use crate::api::state::AppState;
use crate::api::types::ApiError;
use crate::domain::credentials::CredentialType;

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

/// GET /admin/credentials/providers
/// Lists available credential provider types
pub async fn list_credential_providers(
    State(_state): State<AppState>,
    RequireApiKey(api_key): RequireApiKey,
) -> Result<Json<ListCredentialProvidersResponse>, ApiError> {
    if !api_key.permissions().admin {
        return Err(ApiError::forbidden("Admin access required"));
    }

    debug!("Admin listing credential providers");

    let providers = vec![
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
    ];

    Ok(Json(ListCredentialProvidersResponse { providers }))
}

fn credential_type_to_string(ct: &CredentialType) -> String {
    match ct {
        CredentialType::OpenAi => "openai".to_string(),
        CredentialType::Anthropic => "anthropic".to_string(),
        CredentialType::AzureOpenAi => "azure_openai".to_string(),
        CredentialType::AwsBedrock => "aws_bedrock".to_string(),
        CredentialType::Custom(s) => s.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credential_type_to_string() {
        assert_eq!(credential_type_to_string(&CredentialType::OpenAi), "openai");
        assert_eq!(
            credential_type_to_string(&CredentialType::Anthropic),
            "anthropic"
        );
    }
}
