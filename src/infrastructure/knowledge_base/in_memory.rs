//! In-memory knowledge base provider for development and testing

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::domain::knowledge_base::{
    AddDocumentsResult, CreateDocumentRequest, DeleteDocumentsResult, Document, DocumentChunk,
    DocumentSummary, FilterCondition, FilterConnector, FilterOperator, FilterValue,
    KnowledgeBaseDocument, KnowledgeBaseId, KnowledgeBaseProvider, MetadataFilter, SearchParams,
    SearchResult, SourceInfo,
};
use crate::domain::DomainError;
use uuid::Uuid;

/// In-memory knowledge base provider for development without PostgreSQL
#[derive(Debug)]
pub struct InMemoryKnowledgeBaseProvider {
    id: KnowledgeBaseId,
    documents: Arc<RwLock<Vec<StoredDoc>>>,
}

#[derive(Debug, Clone)]
struct StoredDoc {
    id: String,
    content: String,
    metadata: HashMap<String, serde_json::Value>,
    source: Option<String>,
}

impl InMemoryKnowledgeBaseProvider {
    /// Create a new in-memory knowledge base provider
    pub fn new(id: KnowledgeBaseId) -> Self {
        Self {
            id,
            documents: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

#[async_trait]
impl KnowledgeBaseProvider for InMemoryKnowledgeBaseProvider {
    fn knowledge_base_id(&self) -> &KnowledgeBaseId {
        &self.id
    }

    fn provider_type(&self) -> &'static str {
        "in_memory"
    }

    async fn search(&self, params: SearchParams) -> Result<Vec<SearchResult>, DomainError> {
        let docs = self.documents.read().await;
        let query_lower = params.query.to_lowercase();

        // Simple text-based search (substring matching)
        let results: Vec<SearchResult> = docs
            .iter()
            .filter(|doc| doc.content.to_lowercase().contains(&query_lower))
            .take(params.top_k as usize)
            .map(|doc| {
                let mut result = SearchResult::new(&doc.id, &doc.content, 0.8);

                for (key, value) in &doc.metadata {
                    result = result.with_metadata(key, value.clone());
                }

                if let Some(source) = &doc.source {
                    result = result.with_source(source);
                }

                result
            })
            .collect();

        Ok(results)
    }

    async fn add_documents(
        &self,
        documents: Vec<Document>,
    ) -> Result<AddDocumentsResult, DomainError> {
        let mut docs = self.documents.write().await;
        let count = documents.len();

        for doc in documents {
            docs.push(StoredDoc {
                id: doc.id,
                content: doc.content,
                metadata: doc.metadata,
                source: doc.source,
            });
        }

        Ok(AddDocumentsResult::success(count))
    }

    async fn delete_documents(
        &self,
        ids: Vec<String>,
    ) -> Result<DeleteDocumentsResult, DomainError> {
        let mut docs = self.documents.write().await;
        let before = docs.len();

        docs.retain(|doc| !ids.contains(&doc.id));

        let deleted = before - docs.len();
        let not_found = ids.len() - deleted;

        Ok(DeleteDocumentsResult::new(deleted, not_found))
    }

    async fn delete_by_filter(
        &self,
        filter: MetadataFilter,
    ) -> Result<DeleteDocumentsResult, DomainError> {
        let mut docs = self.documents.write().await;
        let before = docs.len();

        docs.retain(|doc| !matches_filter(doc, &filter));

        let deleted = before - docs.len();

        Ok(DeleteDocumentsResult::new(deleted, 0))
    }

    async fn get_document(&self, id: &str) -> Result<Option<SearchResult>, DomainError> {
        let docs = self.documents.read().await;

        let result = docs.iter().find(|doc| doc.id == id).map(|doc| {
            let mut result = SearchResult::new(&doc.id, &doc.content, 1.0);

            for (key, value) in &doc.metadata {
                result = result.with_metadata(key, value.clone());
            }

            if let Some(source) = &doc.source {
                result = result.with_source(source);
            }

            result
        });

        Ok(result)
    }

    async fn health_check(&self) -> Result<bool, DomainError> {
        // In-memory provider is always healthy
        Ok(true)
    }

    async fn delete_by_source(&self, source: &str) -> Result<DeleteDocumentsResult, DomainError> {
        let mut docs = self.documents.write().await;
        let before = docs.len();

        docs.retain(|doc| doc.source.as_deref() != Some(source));

        let deleted = before - docs.len();

        Ok(DeleteDocumentsResult::new(deleted, 0))
    }

    async fn list_sources(&self) -> Result<Vec<SourceInfo>, DomainError> {
        let docs = self.documents.read().await;
        let mut sources: HashMap<String, usize> = HashMap::new();

        for doc in docs.iter() {
            if let Some(source) = &doc.source {
                *sources.entry(source.clone()).or_insert(0) += 1;
            }
        }

        Ok(sources
            .into_iter()
            .map(|(source, count)| SourceInfo {
                source,
                document_count: count,
            })
            .collect())
    }

    async fn list_by_source(&self, source: &str) -> Result<Vec<SearchResult>, DomainError> {
        let docs = self.documents.read().await;

        let results: Vec<SearchResult> = docs
            .iter()
            .filter(|doc| doc.source.as_deref() == Some(source))
            .map(|doc| {
                let mut result = SearchResult::new(&doc.id, &doc.content, 1.0);

                for (key, value) in &doc.metadata {
                    result = result.with_metadata(key, value.clone());
                }

                if let Some(src) = &doc.source {
                    result = result.with_source(src);
                }

                result
            })
            .collect();

        Ok(results)
    }

    async fn document_count(&self) -> Result<usize, DomainError> {
        Ok(self.documents.read().await.len())
    }

    async fn ensure_schema(&self) -> Result<(), DomainError> {
        // No-op for in-memory provider
        Ok(())
    }

    // New document-based methods - not supported in in-memory provider
    async fn create_document(
        &self,
        _request: CreateDocumentRequest,
    ) -> Result<KnowledgeBaseDocument, DomainError> {
        Err(DomainError::knowledge_base(
            "Document-based operations not supported in in-memory provider".to_string(),
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

/// Check if a document matches a metadata filter
fn matches_filter(doc: &StoredDoc, filter: &MetadataFilter) -> bool {
    match filter {
        MetadataFilter::Condition(condition) => matches_condition(doc, condition),
        MetadataFilter::Group { connector, filters } => match connector {
            FilterConnector::And => filters.iter().all(|f| matches_filter(doc, f)),
            FilterConnector::Or => filters.iter().any(|f| matches_filter(doc, f)),
        },
    }
}

/// Check if a document matches a filter condition
fn matches_condition(doc: &StoredDoc, condition: &FilterCondition) -> bool {
    let doc_value = doc.metadata.get(&condition.key);

    match condition.operator {
        FilterOperator::Eq => condition
            .value
            .as_ref()
            .map(|v| compare_eq(doc_value, v))
            .unwrap_or(false),
        FilterOperator::Ne => condition
            .value
            .as_ref()
            .map(|v| !compare_eq(doc_value, v))
            .unwrap_or(true),
        FilterOperator::Gt => condition
            .value
            .as_ref()
            .map(|v| compare_ord(doc_value, v, |a, b| a > b))
            .unwrap_or(false),
        FilterOperator::Gte => condition
            .value
            .as_ref()
            .map(|v| compare_ord(doc_value, v, |a, b| a >= b))
            .unwrap_or(false),
        FilterOperator::Lt => condition
            .value
            .as_ref()
            .map(|v| compare_ord(doc_value, v, |a, b| a < b))
            .unwrap_or(false),
        FilterOperator::Lte => condition
            .value
            .as_ref()
            .map(|v| compare_ord(doc_value, v, |a, b| a <= b))
            .unwrap_or(false),
        FilterOperator::Contains => condition.value.as_ref().map_or(false, |v| {
            if let (Some(doc_str), FilterValue::String(filter_str)) =
                (doc_value.and_then(|v| v.as_str()), v)
            {
                doc_str.contains(filter_str)
            } else {
                false
            }
        }),
        FilterOperator::StartsWith => condition.value.as_ref().map_or(false, |v| {
            if let (Some(doc_str), FilterValue::String(filter_str)) =
                (doc_value.and_then(|v| v.as_str()), v)
            {
                doc_str.starts_with(filter_str)
            } else {
                false
            }
        }),
        FilterOperator::EndsWith => condition.value.as_ref().map_or(false, |v| {
            if let (Some(doc_str), FilterValue::String(filter_str)) =
                (doc_value.and_then(|v| v.as_str()), v)
            {
                doc_str.ends_with(filter_str)
            } else {
                false
            }
        }),
        FilterOperator::In => condition.value.as_ref().map_or(false, |v| {
            if let FilterValue::List(values) = v {
                values.iter().any(|val| compare_eq(doc_value, val))
            } else {
                false
            }
        }),
        FilterOperator::NotIn => condition.value.as_ref().map_or(true, |v| {
            if let FilterValue::List(values) = v {
                !values.iter().any(|val| compare_eq(doc_value, val))
            } else {
                true
            }
        }),
        FilterOperator::Exists => doc_value.is_some(),
        FilterOperator::NotExists => doc_value.is_none(),
    }
}

/// Compare a document value with a filter value for equality
fn compare_eq(doc_value: Option<&serde_json::Value>, filter_value: &FilterValue) -> bool {
    match (doc_value, filter_value) {
        (Some(serde_json::Value::String(s)), FilterValue::String(fs)) => s == fs,
        (Some(serde_json::Value::Number(n)), FilterValue::Integer(fi)) => {
            n.as_i64().map_or(false, |i| i == *fi)
        }
        (Some(serde_json::Value::Number(n)), FilterValue::Float(ff)) => {
            n.as_f64().map_or(false, |f| (f - ff).abs() < f64::EPSILON)
        }
        (Some(serde_json::Value::Bool(b)), FilterValue::Boolean(fb)) => b == fb,
        (Some(serde_json::Value::Null), FilterValue::Null) => true,
        _ => false,
    }
}

/// Compare a document value with a filter value using a comparison function
fn compare_ord<F>(doc_value: Option<&serde_json::Value>, filter_value: &FilterValue, cmp: F) -> bool
where
    F: Fn(f64, f64) -> bool,
{
    match (doc_value, filter_value) {
        (Some(serde_json::Value::Number(n)), FilterValue::Integer(fi)) => {
            n.as_f64().map_or(false, |f| cmp(f, *fi as f64))
        }
        (Some(serde_json::Value::Number(n)), FilterValue::Float(ff)) => {
            n.as_f64().map_or(false, |f| cmp(f, *ff))
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_add_and_search() {
        let id = KnowledgeBaseId::new("test-kb").unwrap();
        let provider = InMemoryKnowledgeBaseProvider::new(id);

        // Add documents
        let docs = vec![
            Document::new("doc1", "Rust is a systems programming language"),
            Document::new("doc2", "Python is great for data science"),
            Document::new("doc3", "Rust provides memory safety guarantees"),
        ];

        let result = provider.add_documents(docs).await.unwrap();
        assert_eq!(result.added, 3);

        // Search for "Rust"
        let params = SearchParams::new("Rust");
        let results = provider.search(params).await.unwrap();

        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_by_source() {
        let id = KnowledgeBaseId::new("test-kb").unwrap();
        let provider = InMemoryKnowledgeBaseProvider::new(id);

        let docs = vec![
            Document::new("doc1", "Content 1").with_source("source-a"),
            Document::new("doc2", "Content 2").with_source("source-a"),
            Document::new("doc3", "Content 3").with_source("source-b"),
        ];

        provider.add_documents(docs).await.unwrap();
        assert_eq!(provider.document_count().await.unwrap(), 3);

        let result = provider.delete_by_source("source-a").await.unwrap();
        assert_eq!(result.deleted, 2);
        assert_eq!(provider.document_count().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_list_sources() {
        let id = KnowledgeBaseId::new("test-kb").unwrap();
        let provider = InMemoryKnowledgeBaseProvider::new(id);

        let docs = vec![
            Document::new("doc1", "Content 1").with_source("source-a"),
            Document::new("doc2", "Content 2").with_source("source-a"),
            Document::new("doc3", "Content 3").with_source("source-b"),
        ];

        provider.add_documents(docs).await.unwrap();

        let sources = provider.list_sources().await.unwrap();
        assert_eq!(sources.len(), 2);
    }

    #[tokio::test]
    async fn test_get_document() {
        let id = KnowledgeBaseId::new("test-kb").unwrap();
        let provider = InMemoryKnowledgeBaseProvider::new(id);

        let docs = vec![Document::new("doc1", "Test content")];
        provider.add_documents(docs).await.unwrap();

        let doc = provider.get_document("doc1").await.unwrap();
        assert!(doc.is_some());
        assert_eq!(doc.unwrap().content, "Test content");

        let missing = provider.get_document("nonexistent").await.unwrap();
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn test_health_check() {
        let id = KnowledgeBaseId::new("test-kb").unwrap();
        let provider = InMemoryKnowledgeBaseProvider::new(id);

        assert!(provider.health_check().await.unwrap());
    }
}
