//! AWS Bedrock Knowledge Base provider implementation

use std::collections::HashMap;
use std::fmt::Debug;

use async_trait::async_trait;
use aws_sdk_bedrockagentruntime::Client as BedrockAgentClient;
use aws_smithy_types::Document as SmithyDocument;

use crate::domain::knowledge_base::{
    AddDocumentsResult, CreateDocumentRequest, DeleteDocumentsResult, Document, DocumentChunk,
    DocumentSummary, KnowledgeBaseDocument, KnowledgeBaseId, KnowledgeBaseProvider, MetadataFilter,
    SearchParams, SearchResult, SourceInfo,
};
use crate::domain::DomainError;
use uuid::Uuid;

/// Configuration for AWS Bedrock Knowledge Base
#[derive(Debug, Clone)]
pub struct AwsKnowledgeBaseConfig {
    /// AWS Knowledge Base ID (from Bedrock console)
    pub aws_kb_id: String,
    /// Model ARN for retrieval (optional)
    pub model_arn: Option<String>,
    /// AWS region
    pub region: Option<String>,
}

impl AwsKnowledgeBaseConfig {
    /// Create a new AWS Knowledge Base configuration
    pub fn new(aws_kb_id: impl Into<String>) -> Self {
        Self {
            aws_kb_id: aws_kb_id.into(),
            model_arn: None,
            region: None,
        }
    }

    /// Set the model ARN for retrieval
    pub fn with_model_arn(mut self, arn: impl Into<String>) -> Self {
        self.model_arn = Some(arn.into());
        self
    }

    /// Set the AWS region
    pub fn with_region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }
}

/// AWS Bedrock Knowledge Base provider
pub struct AwsKnowledgeBase {
    id: KnowledgeBaseId,
    config: AwsKnowledgeBaseConfig,
    client: BedrockAgentClient,
}

impl Debug for AwsKnowledgeBase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AwsKnowledgeBase")
            .field("id", &self.id)
            .field("config", &self.config)
            .finish()
    }
}

impl AwsKnowledgeBase {
    /// Create a new AWS Knowledge Base provider
    pub async fn new(
        id: KnowledgeBaseId,
        config: AwsKnowledgeBaseConfig,
    ) -> Result<Self, DomainError> {
        let aws_config = if let Some(region) = &config.region {
            aws_config::defaults(aws_config::BehaviorVersion::latest())
                .region(aws_config::Region::new(region.clone()))
                .load()
                .await
        } else {
            aws_config::defaults(aws_config::BehaviorVersion::latest())
                .load()
                .await
        };

        let client = BedrockAgentClient::new(&aws_config);

        Ok(Self { id, config, client })
    }

    /// Create with an existing AWS SDK config
    pub fn with_config(
        id: KnowledgeBaseId,
        config: AwsKnowledgeBaseConfig,
        aws_config: &aws_config::SdkConfig,
    ) -> Self {
        let client = BedrockAgentClient::new(aws_config);
        Self { id, config, client }
    }

    /// Convert MetadataFilter to AWS retrieval filter
    fn build_retrieval_filter(
        &self,
        filter: &MetadataFilter,
    ) -> Option<aws_sdk_bedrockagentruntime::types::RetrievalFilter> {
        use aws_sdk_bedrockagentruntime::types::{FilterAttribute, RetrievalFilter};

        match filter {
            MetadataFilter::Condition(condition) => {
                let key = condition.key.clone();

                if let Some(value) = &condition.value {
                    let doc_value = self.filter_value_to_doc(value);

                    let filter_attr = FilterAttribute::builder().key(key).value(doc_value).build();

                    match filter_attr {
                        Ok(attr) => match &condition.operator {
                            crate::domain::knowledge_base::FilterOperator::Eq => {
                                Some(RetrievalFilter::Equals(attr))
                            }
                            crate::domain::knowledge_base::FilterOperator::Ne => {
                                Some(RetrievalFilter::NotEquals(attr))
                            }
                            crate::domain::knowledge_base::FilterOperator::Gt => {
                                Some(RetrievalFilter::GreaterThan(attr))
                            }
                            crate::domain::knowledge_base::FilterOperator::Gte => {
                                Some(RetrievalFilter::GreaterThanOrEquals(attr))
                            }
                            crate::domain::knowledge_base::FilterOperator::Lt => {
                                Some(RetrievalFilter::LessThan(attr))
                            }
                            crate::domain::knowledge_base::FilterOperator::Lte => {
                                Some(RetrievalFilter::LessThanOrEquals(attr))
                            }
                            crate::domain::knowledge_base::FilterOperator::In => {
                                Some(RetrievalFilter::In(attr))
                            }
                            crate::domain::knowledge_base::FilterOperator::NotIn => {
                                Some(RetrievalFilter::NotIn(attr))
                            }
                            crate::domain::knowledge_base::FilterOperator::StartsWith => {
                                Some(RetrievalFilter::StartsWith(attr))
                            }
                            crate::domain::knowledge_base::FilterOperator::Contains => {
                                Some(RetrievalFilter::StringContains(attr))
                            }
                            _ => None,
                        },
                        Err(_) => None,
                    }
                } else {
                    None
                }
            }
            MetadataFilter::Group { connector, filters } => {
                let sub_filters: Vec<RetrievalFilter> = filters
                    .iter()
                    .filter_map(|f| self.build_retrieval_filter(f))
                    .collect();

                if sub_filters.is_empty() {
                    return None;
                }

                match connector {
                    crate::domain::knowledge_base::FilterConnector::And => {
                        Some(RetrievalFilter::AndAll(sub_filters))
                    }
                    crate::domain::knowledge_base::FilterConnector::Or => {
                        Some(RetrievalFilter::OrAll(sub_filters))
                    }
                }
            }
        }
    }

    fn filter_value_to_doc(
        &self,
        value: &crate::domain::knowledge_base::FilterValue,
    ) -> SmithyDocument {
        use crate::domain::knowledge_base::FilterValue;

        match value {
            FilterValue::String(s) => SmithyDocument::String(s.clone()),
            FilterValue::Integer(n) => {
                SmithyDocument::Number(aws_smithy_types::Number::PosInt(*n as u64))
            }
            FilterValue::Float(f) => SmithyDocument::Number(aws_smithy_types::Number::Float(*f)),
            FilterValue::Boolean(b) => SmithyDocument::Bool(*b),
            FilterValue::Null => SmithyDocument::Null,
            FilterValue::List(items) => {
                let docs: Vec<SmithyDocument> =
                    items.iter().map(|v| self.filter_value_to_doc(v)).collect();
                SmithyDocument::Array(docs)
            }
        }
    }
}

#[async_trait]
impl KnowledgeBaseProvider for AwsKnowledgeBase {
    fn knowledge_base_id(&self) -> &KnowledgeBaseId {
        &self.id
    }

    fn provider_type(&self) -> &'static str {
        "aws_knowledge_base"
    }

    async fn search(&self, params: SearchParams) -> Result<Vec<SearchResult>, DomainError> {
        use aws_sdk_bedrockagentruntime::types::{
            KnowledgeBaseQuery, KnowledgeBaseRetrievalConfiguration,
            KnowledgeBaseVectorSearchConfiguration,
        };

        let mut vector_config_builder = KnowledgeBaseVectorSearchConfiguration::builder()
            .number_of_results(params.top_k as i32);

        if let Some(filter) = &params.filter {
            if let Some(retrieval_filter) = self.build_retrieval_filter(filter) {
                vector_config_builder = vector_config_builder.filter(retrieval_filter);
            }
        }

        let vector_config = vector_config_builder.build();

        let retrieval_config = KnowledgeBaseRetrievalConfiguration::builder()
            .vector_search_configuration(vector_config)
            .build();

        let query = KnowledgeBaseQuery::builder()
            .text(params.query)
            .build();

        let mut request = self
            .client
            .retrieve()
            .knowledge_base_id(&self.config.aws_kb_id)
            .retrieval_query(query)
            .retrieval_configuration(retrieval_config);

        if let Some(ref model_arn) = self.config.model_arn {
            request = request.guardrail_configuration(
                aws_sdk_bedrockagentruntime::types::GuardrailConfiguration::builder()
                    .guardrail_id(model_arn.clone())
                    .build()
                    .map_err(|e| {
                        DomainError::knowledge_base(format!(
                            "Failed to build guardrail config: {}",
                            e
                        ))
                    })?,
            );
        }

        let response = request.send().await.map_err(|e| {
            DomainError::knowledge_base(format!("AWS Knowledge Base search failed: {}", e))
        })?;

        let mut results: Vec<SearchResult> = Vec::new();

        for r in response.retrieval_results() {
            let content = match r.content() {
                Some(c) => c.text().to_string(),
                None => continue,
            };

            let score = r.score().unwrap_or(0.0) as f32;

            if score < params.similarity_threshold {
                continue;
            }

            let id = r
                .location()
                .and_then(|l| l.s3_location())
                .map(|s3| s3.uri().unwrap_or("unknown").to_string())
                .unwrap_or_else(|| format!("result-{}", score));

            let mut result = SearchResult::new(&id, &content, score);

            if let Some(metadata) = r.metadata() {
                let mut meta_map: HashMap<String, serde_json::Value> = HashMap::new();

                for (key, doc) in metadata {
                    if let Some(val) = doc_to_json(doc) {
                        meta_map.insert(key.clone(), val);
                    }
                }

                result = result.with_all_metadata(meta_map);
            }

            if let Some(loc) = r.location() {
                if let Some(s3) = loc.s3_location() {
                    if let Some(uri) = s3.uri() {
                        result = result.with_source(uri.to_string());
                    }
                }
            }

            results.push(result);
        }

        Ok(results)
    }

    async fn add_documents(
        &self,
        _documents: Vec<Document>,
    ) -> Result<AddDocumentsResult, DomainError> {
        // AWS Knowledge Bases are managed through the Bedrock console or S3 sync
        // Direct document ingestion is not supported through the runtime API
        Err(DomainError::knowledge_base(
            "AWS Knowledge Base does not support direct document ingestion. \
             Use S3 data source sync instead."
                .to_string(),
        ))
    }

    async fn delete_documents(
        &self,
        _ids: Vec<String>,
    ) -> Result<DeleteDocumentsResult, DomainError> {
        // AWS Knowledge Bases are managed through the Bedrock console
        Err(DomainError::knowledge_base(
            "AWS Knowledge Base does not support direct document deletion. \
             Use S3 data source management instead."
                .to_string(),
        ))
    }

    async fn delete_by_filter(
        &self,
        _filter: MetadataFilter,
    ) -> Result<DeleteDocumentsResult, DomainError> {
        Err(DomainError::knowledge_base(
            "AWS Knowledge Base does not support direct document deletion. \
             Use S3 data source management instead."
                .to_string(),
        ))
    }

    async fn get_document(&self, _id: &str) -> Result<Option<SearchResult>, DomainError> {
        // AWS Knowledge Bases don't provide direct document access
        Err(DomainError::knowledge_base(
            "AWS Knowledge Base does not support direct document retrieval. \
             Use search instead."
                .to_string(),
        ))
    }

    async fn health_check(&self) -> Result<bool, DomainError> {
        // Try a minimal search to verify connectivity
        let params = SearchParams::new("health check").with_top_k(1);

        match self.search(params).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    async fn document_count(&self) -> Result<usize, DomainError> {
        // AWS Knowledge Bases don't expose document count through the runtime API
        Err(DomainError::knowledge_base(
            "AWS Knowledge Base does not support document count. \
             Use AWS console or management API instead."
                .to_string(),
        ))
    }

    async fn list_by_source(&self, _source: &str) -> Result<Vec<SearchResult>, DomainError> {
        Err(DomainError::knowledge_base(
            "AWS Knowledge Base does not support listing documents by source. \
             Use AWS console or S3 browser instead."
                .to_string(),
        ))
    }

    async fn delete_by_source(&self, _source: &str) -> Result<DeleteDocumentsResult, DomainError> {
        Err(DomainError::knowledge_base(
            "AWS Knowledge Base does not support deleting documents by source. \
             Use S3 data source management instead."
                .to_string(),
        ))
    }

    async fn list_sources(&self) -> Result<Vec<SourceInfo>, DomainError> {
        Err(DomainError::knowledge_base(
            "AWS Knowledge Base does not support listing sources. \
             Use AWS console or S3 browser instead."
                .to_string(),
        ))
    }

    async fn ensure_schema(&self) -> Result<(), DomainError> {
        // AWS Knowledge Bases are managed through the Bedrock console
        Ok(())
    }

    // New document-based methods - not supported in AWS provider
    async fn create_document(
        &self,
        _request: CreateDocumentRequest,
    ) -> Result<KnowledgeBaseDocument, DomainError> {
        Err(DomainError::knowledge_base(
            "Document creation not supported in AWS Knowledge Base. Use S3 data source.".to_string(),
        ))
    }

    async fn get_document_by_id(&self, _id: Uuid) -> Result<Option<KnowledgeBaseDocument>, DomainError> {
        Ok(None)
    }

    async fn list_documents(&self) -> Result<Vec<DocumentSummary>, DomainError> {
        Ok(Vec::new())
    }

    async fn get_document_chunks(&self, _document_id: Uuid) -> Result<Vec<DocumentChunk>, DomainError> {
        Ok(Vec::new())
    }

    async fn delete_document_by_id(&self, _id: Uuid) -> Result<bool, DomainError> {
        Ok(false)
    }

    async fn disable_document(&self, _id: Uuid) -> Result<bool, DomainError> {
        Ok(false)
    }

    async fn enable_document(&self, _id: Uuid) -> Result<bool, DomainError> {
        Ok(false)
    }
}

/// Convert AWS Smithy Document to serde_json::Value
fn doc_to_json(doc: &SmithyDocument) -> Option<serde_json::Value> {
    match doc {
        SmithyDocument::String(s) => Some(serde_json::Value::String(s.clone())),
        SmithyDocument::Number(n) => {
            let f = n.to_f64_lossy();
            Some(serde_json::json!(f))
        }
        SmithyDocument::Bool(b) => Some(serde_json::Value::Bool(*b)),
        SmithyDocument::Null => Some(serde_json::Value::Null),
        SmithyDocument::Array(arr) => {
            let values: Vec<serde_json::Value> = arr.iter().filter_map(doc_to_json).collect();
            Some(serde_json::Value::Array(values))
        }
        SmithyDocument::Object(obj) => {
            let mut map = serde_json::Map::new();

            for (k, v) in obj {
                if let Some(val) = doc_to_json(v) {
                    map.insert(k.clone(), val);
                }
            }

            Some(serde_json::Value::Object(map))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aws_kb_config() {
        let config = AwsKnowledgeBaseConfig::new("kb-12345")
            .with_model_arn("arn:aws:bedrock:us-east-1::foundation-model/anthropic.claude-v2")
            .with_region("us-west-2");

        assert_eq!(config.aws_kb_id, "kb-12345");
        assert!(config.model_arn.is_some());
        assert_eq!(config.region, Some("us-west-2".to_string()));
    }
}
