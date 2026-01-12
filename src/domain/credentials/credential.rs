use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Type of credential (which LLM provider or service it belongs to)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialType {
    // LLM Providers
    OpenAi,
    Anthropic,
    AzureOpenAi,
    AwsBedrock,
    // Knowledge Base Providers
    Pgvector,
    AwsKnowledgeBase,
    Pinecone,
    // HTTP API Credential
    /// API key for external HTTP APIs (used by HTTP Request workflow steps)
    HttpApiKey,
    // Custom
    Custom(String),
}

/// Credential entity containing API keys and secrets
#[derive(Debug, Clone)]
pub struct Credential {
    credential_type: CredentialType,
    api_key: String,
    additional_params: std::collections::HashMap<String, String>,
    expires_at: Option<DateTime<Utc>>,
    fetched_at: DateTime<Utc>,
}

impl Credential {
    pub fn new(credential_type: CredentialType, api_key: String) -> Self {
        Self {
            credential_type,
            api_key,
            additional_params: std::collections::HashMap::new(),
            expires_at: None,
            fetched_at: Utc::now(),
        }
    }

    pub fn with_expiration(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    pub fn with_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.additional_params.insert(key.into(), value.into());
        self
    }

    pub fn credential_type(&self) -> &CredentialType {
        &self.credential_type
    }

    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    pub fn get_param(&self, key: &str) -> Option<&String> {
        self.additional_params.get(key)
    }

    pub fn additional_params(&self) -> &std::collections::HashMap<String, String> {
        &self.additional_params
    }

    pub fn expires_at(&self) -> Option<DateTime<Utc>> {
        self.expires_at
    }

    pub fn fetched_at(&self) -> DateTime<Utc> {
        self.fetched_at
    }

    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|exp| exp < Utc::now())
            .unwrap_or(false)
    }
}

impl std::fmt::Display for CredentialType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CredentialType::OpenAi => write!(f, "openai"),
            CredentialType::Anthropic => write!(f, "anthropic"),
            CredentialType::AzureOpenAi => write!(f, "azure_openai"),
            CredentialType::AwsBedrock => write!(f, "aws_bedrock"),
            CredentialType::Pgvector => write!(f, "pgvector"),
            CredentialType::AwsKnowledgeBase => write!(f, "aws_knowledge_base"),
            CredentialType::Pinecone => write!(f, "pinecone"),
            CredentialType::HttpApiKey => write!(f, "http_api_key"),
            CredentialType::Custom(name) => write!(f, "custom:{}", name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credential_creation() {
        let cred = Credential::new(CredentialType::OpenAi, "sk-test-key".to_string());

        assert_eq!(cred.credential_type(), &CredentialType::OpenAi);
        assert_eq!(cred.api_key(), "sk-test-key");
        assert!(!cred.is_expired());
    }

    #[test]
    fn test_credential_with_params() {
        let cred = Credential::new(CredentialType::AzureOpenAi, "key".to_string())
            .with_param("endpoint", "https://example.openai.azure.com")
            .with_param("deployment", "gpt-4");

        assert_eq!(
            cred.get_param("endpoint"),
            Some(&"https://example.openai.azure.com".to_string())
        );
        assert_eq!(cred.get_param("deployment"), Some(&"gpt-4".to_string()));
    }

    #[test]
    fn test_credential_expiration() {
        let past = Utc::now() - chrono::Duration::hours(1);
        let cred = Credential::new(CredentialType::OpenAi, "key".to_string())
            .with_expiration(past);

        assert!(cred.is_expired());
    }
}
