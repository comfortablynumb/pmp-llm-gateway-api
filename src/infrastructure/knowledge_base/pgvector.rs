//! pgvector knowledge base provider implementation

use std::collections::HashMap;
use std::fmt::Debug;

use async_trait::async_trait;
use sqlx::postgres::PgPool;
use sqlx::Row;

use crate::domain::knowledge_base::{
    AddDocumentsResult, CreateDocumentRequest, DeleteDocumentsResult, Document, DocumentChunk,
    DocumentSummary, FilterCondition, FilterConnector, FilterOperator, FilterValue,
    KnowledgeBaseDocument, KnowledgeBaseId, KnowledgeBaseProvider, MetadataFilter, SearchParams,
    SearchResult, SourceInfo,
};
use crate::domain::DomainError;
use uuid::Uuid;

/// Configuration for pgvector knowledge base
#[derive(Debug, Clone)]
pub struct PgvectorConfig {
    /// Embedding dimensions
    pub dimensions: u32,
    /// Table name for storing vectors
    pub table_name: String,
    /// Distance metric to use
    pub distance_metric: DistanceMetric,
}

impl PgvectorConfig {
    /// Create a new pgvector configuration
    pub fn new(dimensions: u32) -> Self {
        Self {
            dimensions,
            table_name: "knowledge_base_document_chunks".to_string(),
            distance_metric: DistanceMetric::Cosine,
        }
    }

    /// Set the table name
    pub fn with_table_name(mut self, name: impl Into<String>) -> Self {
        self.table_name = name.into();
        self
    }

    /// Set the distance metric
    pub fn with_distance_metric(mut self, metric: DistanceMetric) -> Self {
        self.distance_metric = metric;
        self
    }
}

/// Distance metric for vector similarity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DistanceMetric {
    /// Cosine distance (1 - cosine similarity)
    Cosine,
    /// Euclidean (L2) distance
    Euclidean,
    /// Inner product (negative dot product for similarity)
    InnerProduct,
}

impl DistanceMetric {
    /// Get the pgvector operator for this metric
    fn operator(&self) -> &'static str {
        match self {
            Self::Cosine => "<=>",
            Self::Euclidean => "<->",
            Self::InnerProduct => "<#>",
        }
    }

    /// Convert distance to similarity score (0-1)
    fn to_similarity(&self, distance: f64) -> f32 {
        match self {
            Self::Cosine => (1.0 - distance) as f32,
            Self::Euclidean => {
                // Convert to similarity: 1 / (1 + distance)
                (1.0 / (1.0 + distance)) as f32
            }
            Self::InnerProduct => {
                // Inner product is already a similarity (negated in pgvector)
                (-distance) as f32
            }
        }
    }
}

/// Trait for embedding providers (to generate embeddings from text)
#[async_trait]
pub trait EmbeddingProvider: Send + Sync + Debug {
    /// Generate embeddings for the given texts
    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, DomainError>;

    /// Get the embedding dimensions
    fn dimensions(&self) -> u32;
}

/// pgvector-based knowledge base provider
pub struct PgvectorKnowledgeBase<E: EmbeddingProvider> {
    id: KnowledgeBaseId,
    pool: PgPool,
    config: PgvectorConfig,
    embedding_provider: E,
}

impl<E: EmbeddingProvider> Debug for PgvectorKnowledgeBase<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PgvectorKnowledgeBase")
            .field("id", &self.id)
            .field("config", &self.config)
            .field("embedding_provider", &self.embedding_provider)
            .finish()
    }
}

impl<E: EmbeddingProvider> PgvectorKnowledgeBase<E> {
    /// Create a new pgvector knowledge base
    pub fn new(
        id: KnowledgeBaseId,
        pool: PgPool,
        config: PgvectorConfig,
        embedding_provider: E,
    ) -> Self {
        Self {
            id,
            pool,
            config,
            embedding_provider,
        }
    }

    /// Ensure the vector table exists with pgvector extension
    pub async fn ensure_table(&self) -> Result<(), DomainError> {
        // Create pgvector extension if not exists
        sqlx::query("CREATE EXTENSION IF NOT EXISTS vector")
            .execute(&self.pool)
            .await
            .map_err(|e| {
                DomainError::knowledge_base(format!("Failed to create vector extension: {}", e))
            })?;

        // Create the table
        let query = format!(
            r#"
            CREATE TABLE IF NOT EXISTS {} (
                id VARCHAR(255) PRIMARY KEY,
                kb_id VARCHAR(50) NOT NULL,
                content TEXT NOT NULL,
                embedding vector({}) NOT NULL,
                metadata JSONB DEFAULT '{{}}',
                source VARCHAR(1000),
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
            "#,
            self.config.table_name, self.config.dimensions
        );

        sqlx::query(&query).execute(&self.pool).await.map_err(|e| {
            DomainError::knowledge_base(format!("Failed to create table: {}", e))
        })?;

        // Create index on kb_id
        let index_query = format!(
            "CREATE INDEX IF NOT EXISTS idx_{}_kb_id ON {} (kb_id)",
            self.config.table_name, self.config.table_name
        );

        sqlx::query(&index_query)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                DomainError::knowledge_base(format!("Failed to create kb_id index: {}", e))
            })?;

        // Create vector index for similarity search
        let vector_index = format!(
            "CREATE INDEX IF NOT EXISTS idx_{}_embedding ON {} USING ivfflat (embedding vector_cosine_ops)",
            self.config.table_name,
            self.config.table_name
        );

        // IVFFlat requires some data to build, so ignore errors
        let _ = sqlx::query(&vector_index).execute(&self.pool).await;

        Ok(())
    }

    /// Convert a filter to SQL WHERE clause
    /// Values are embedded directly in the SQL string for simplicity
    /// `table_alias` is an optional table alias prefix (e.g., "c" for "c.metadata")
    fn filter_to_sql(&self, filter: &MetadataFilter, table_alias: Option<&str>) -> String {
        match filter {
            MetadataFilter::Condition(condition) => self.condition_to_sql(condition, table_alias),
            MetadataFilter::Group { connector, filters } => {
                let clauses: Vec<String> = filters
                    .iter()
                    .map(|f| self.filter_to_sql(f, table_alias))
                    .filter(|s| !s.is_empty())
                    .collect();

                if clauses.is_empty() {
                    return String::new();
                }

                let connector_sql = match connector {
                    FilterConnector::And => " AND ",
                    FilterConnector::Or => " OR ",
                };

                format!("({})", clauses.join(connector_sql))
            }
        }
    }

    fn condition_to_sql(
        &self,
        condition: &FilterCondition,
        table_alias: Option<&str>,
    ) -> String {
        let key = &condition.key;
        let metadata_col = match table_alias {
            Some(alias) => format!("{}.metadata", alias),
            None => "metadata".to_string(),
        };
        let json_path = format!("{}->'{}'", metadata_col, key);

        match &condition.operator {
            FilterOperator::Exists => {
                format!("{} ? '{}'", metadata_col, key)
            }
            FilterOperator::NotExists => {
                format!("NOT ({} ? '{}')", metadata_col, key)
            }
            _ => {
                if let Some(value) = &condition.value {
                    let (op, val_sql) =
                        self.operator_and_value_sql(&condition.operator, value);
                    format!("{} {} {}", json_path, op, val_sql)
                } else {
                    String::new()
                }
            }
        }
    }

    fn operator_and_value_sql(
        &self,
        op: &FilterOperator,
        value: &FilterValue,
    ) -> (&'static str, String) {
        match op {
            FilterOperator::Eq => {
                let val = self.filter_value_to_json(value);
                ("=", format!("'{}'::jsonb", val))
            }
            FilterOperator::Ne => {
                let val = self.filter_value_to_json(value);
                ("!=", format!("'{}'::jsonb", val))
            }
            FilterOperator::Gt => {
                let val = self.filter_value_to_numeric_string(value);
                (
                    ">",
                    format!("('{}'::text)::numeric", val),
                )
            }
            FilterOperator::Gte => {
                let val = self.filter_value_to_numeric_string(value);
                (
                    ">=",
                    format!("('{}'::text)::numeric", val),
                )
            }
            FilterOperator::Lt => {
                let val = self.filter_value_to_numeric_string(value);
                (
                    "<",
                    format!("('{}'::text)::numeric", val),
                )
            }
            FilterOperator::Lte => {
                let val = self.filter_value_to_numeric_string(value);
                (
                    "<=",
                    format!("('{}'::text)::numeric", val),
                )
            }
            FilterOperator::Contains => {
                if let FilterValue::String(s) = value {
                    // Escape single quotes for SQL
                    let escaped = s.replace('\'', "''");
                    (
                        "LIKE",
                        format!("'%{}%'", escaped),
                    )
                } else {
                    ("=", "NULL".to_string())
                }
            }
            FilterOperator::StartsWith => {
                if let FilterValue::String(s) = value {
                    let escaped = s.replace('\'', "''");
                    (
                        "LIKE",
                        format!("'{}%'", escaped),
                    )
                } else {
                    ("=", "NULL".to_string())
                }
            }
            FilterOperator::EndsWith => {
                if let FilterValue::String(s) = value {
                    let escaped = s.replace('\'', "''");
                    (
                        "LIKE",
                        format!("'%{}'", escaped),
                    )
                } else {
                    ("=", "NULL".to_string())
                }
            }
            FilterOperator::In => {
                if let FilterValue::List(items) = value {
                    let json_array: Vec<String> = items
                        .iter()
                        .map(|v| self.filter_value_to_json(v))
                        .collect();
                    let array_str = format!("[{}]", json_array.join(","));
                    return (
                        "= ANY",
                        format!("(SELECT jsonb_array_elements('{}'::jsonb))", array_str),
                    );
                }
                ("=", "NULL".to_string())
            }
            FilterOperator::NotIn => {
                if let FilterValue::List(items) = value {
                    let json_array: Vec<String> = items
                        .iter()
                        .map(|v| self.filter_value_to_json(v))
                        .collect();
                    let array_str = format!("[{}]", json_array.join(","));
                    return (
                        "NOT IN",
                        format!("(SELECT jsonb_array_elements('{}'::jsonb))", array_str),
                    );
                }
                ("=", "NULL".to_string())
            }
            FilterOperator::Exists | FilterOperator::NotExists => {
                // Handled separately in condition_to_sql
                ("=", "NULL".to_string())
            }
        }
    }

    fn filter_value_to_json(&self, value: &FilterValue) -> String {
        match value {
            FilterValue::String(s) => format!("\"{}\"", s),
            FilterValue::Integer(n) => n.to_string(),
            FilterValue::Float(f) => f.to_string(),
            FilterValue::Boolean(b) => b.to_string(),
            FilterValue::Null => "null".to_string(),
            FilterValue::List(items) => {
                let json_items: Vec<String> =
                    items.iter().map(|v| self.filter_value_to_json(v)).collect();
                format!("[{}]", json_items.join(","))
            }
        }
    }

    fn filter_value_to_numeric_string(&self, value: &FilterValue) -> String {
        match value {
            FilterValue::Integer(n) => n.to_string(),
            FilterValue::Float(f) => f.to_string(),
            _ => "0".to_string(),
        }
    }

    fn embedding_to_pgvector(&self, embedding: &[f32]) -> String {
        let values: Vec<String> = embedding.iter().map(|v| v.to_string()).collect();
        format!("[{}]", values.join(","))
    }
}

#[async_trait]
impl<E: EmbeddingProvider + 'static> KnowledgeBaseProvider for PgvectorKnowledgeBase<E> {
    fn knowledge_base_id(&self) -> &KnowledgeBaseId {
        &self.id
    }

    fn provider_type(&self) -> &'static str {
        "pgvector"
    }

    async fn search(&self, params: SearchParams) -> Result<Vec<SearchResult>, DomainError> {
        tracing::debug!(
            kb_id = self.id.as_str(),
            query = %params.query,
            top_k = params.top_k,
            similarity_threshold = params.similarity_threshold,
            has_filter = params.filter.is_some(),
            "Starting KB search"
        );

        // Generate embedding for the query
        let embeddings = self
            .embedding_provider
            .embed(vec![params.query.clone()])
            .await?;

        let query_embedding = embeddings.into_iter().next().ok_or_else(|| {
            DomainError::knowledge_base("Failed to generate query embedding".to_string())
        })?;

        tracing::debug!(
            kb_id = self.id.as_str(),
            embedding_dimensions = query_embedding.len(),
            "Generated query embedding"
        );

        let embedding_str = self.embedding_to_pgvector(&query_embedding);
        let op = self.config.distance_metric.operator();

        // Build filter SQL if filter is provided (use "c" as table alias for chunks table)
        let filter_sql = params
            .filter
            .as_ref()
            .map(|f| self.filter_to_sql(f, Some("c")))
            .filter(|s| !s.is_empty())
            .map(|s| format!(" AND {}", s))
            .unwrap_or_default();

        if !filter_sql.is_empty() {
            tracing::debug!(
                kb_id = self.id.as_str(),
                filter_sql = %filter_sql,
                "Applying metadata filter to search"
            );
        }

        // Query the new schema (knowledge_base_document_chunks) which is used by document ingestion
        // Join with documents table to exclude disabled documents
        let query = format!(
            r#"
            SELECT
                c.id::text as id,
                c.content,
                c.embedding {} '{}' as distance,
                c.metadata,
                d.source_filename as source,
                c.embedding::text as embedding
            FROM knowledge_base_document_chunks c
            JOIN knowledge_base_documents d ON c.document_id = d.id
            WHERE c.kb_id = '{}'
              AND (d.disabled IS NULL OR d.disabled = false){}
            ORDER BY distance
            LIMIT {}
            "#,
            op, embedding_str, self.id.as_str(), filter_sql, params.top_k
        );

        let rows = sqlx::query(&query)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!(
                    kb_id = self.id.as_str(),
                    error = %e,
                    query = %query,
                    "KB search failed"
                );
                DomainError::knowledge_base(format!("Search failed: {}", e))
            })?;

        tracing::debug!(
            kb_id = self.id.as_str(),
            rows_from_db = rows.len(),
            "Retrieved rows from database"
        );

        let mut results = Vec::with_capacity(rows.len());
        let mut filtered_count = 0;

        for row in rows {
            let id: String = row.get("id");
            let content: String = row.get("content");
            let distance: f64 = row.get("distance");
            let metadata: serde_json::Value = row.get("metadata");
            let source: Option<String> = row.get("source");

            let score = self.config.distance_metric.to_similarity(distance);

            if score < params.similarity_threshold {
                filtered_count += 1;
                continue;
            }

            let metadata_map: HashMap<String, serde_json::Value> =
                serde_json::from_value(metadata).unwrap_or_default();

            let mut result = SearchResult::new(&id, &content, score).with_all_metadata(metadata_map);

            if let Some(src) = source {
                result = result.with_source(src);
            }

            if params.include_embeddings {
                let emb_str: String = row.get("embedding");
                if let Ok(emb) = parse_pgvector(&emb_str) {
                    result = result.with_embedding(emb);
                }
            }

            results.push(result);
        }

        tracing::debug!(
            kb_id = self.id.as_str(),
            results_after_filter = results.len(),
            filtered_by_threshold = filtered_count,
            similarity_threshold = params.similarity_threshold,
            "Search completed"
        );

        Ok(results)
    }

    async fn add_documents(
        &self,
        documents: Vec<Document>,
    ) -> Result<AddDocumentsResult, DomainError> {
        if documents.is_empty() {
            return Ok(AddDocumentsResult::success(0));
        }

        // Generate embeddings for all documents
        let texts: Vec<String> = documents.iter().map(|d| d.content.clone()).collect();
        let embeddings = self.embedding_provider.embed(texts).await?;

        if embeddings.len() != documents.len() {
            return Err(DomainError::knowledge_base(
                "Embedding count mismatch".to_string(),
            ));
        }

        let mut added = 0;
        let mut errors: Vec<(String, String)> = Vec::new();

        for (doc, embedding) in documents.into_iter().zip(embeddings.into_iter()) {
            let embedding_str = self.embedding_to_pgvector(&embedding);
            let metadata = serde_json::to_value(&doc.metadata).unwrap_or_default();

            let query = format!(
                r#"
                INSERT INTO {} (id, kb_id, content, embedding, metadata, source)
                VALUES ($1, $2, $3, '{}'::vector, $4, $5)
                ON CONFLICT (id) DO UPDATE SET
                    content = EXCLUDED.content,
                    embedding = EXCLUDED.embedding,
                    metadata = EXCLUDED.metadata,
                    source = EXCLUDED.source,
                    updated_at = NOW()
                "#,
                self.config.table_name, embedding_str
            );

            let result = sqlx::query(&query)
                .bind(&doc.id)
                .bind(self.id.as_str())
                .bind(&doc.content)
                .bind(&metadata)
                .bind(&doc.source)
                .execute(&self.pool)
                .await;

            match result {
                Ok(_) => added += 1,
                Err(e) => errors.push((doc.id, e.to_string())),
            }
        }

        if errors.is_empty() {
            Ok(AddDocumentsResult::success(added))
        } else {
            Ok(AddDocumentsResult::partial(added, errors))
        }
    }

    async fn delete_documents(
        &self,
        ids: Vec<String>,
    ) -> Result<DeleteDocumentsResult, DomainError> {
        if ids.is_empty() {
            return Ok(DeleteDocumentsResult::new(0, 0));
        }

        // Build placeholders for IN clause
        let placeholders: Vec<String> = (1..=ids.len()).map(|i| format!("${}", i)).collect();

        let query = format!(
            "DELETE FROM {} WHERE kb_id = '{}' AND id IN ({})",
            self.config.table_name,
            self.id.as_str(),
            placeholders.join(", ")
        );

        let mut query_builder = sqlx::query(&query);

        for id in &ids {
            query_builder = query_builder.bind(id);
        }

        let result = query_builder.execute(&self.pool).await.map_err(|e| {
            DomainError::knowledge_base(format!("Failed to delete documents: {}", e))
        })?;

        let deleted = result.rows_affected() as usize;
        let not_found = ids.len() - deleted;

        Ok(DeleteDocumentsResult::new(deleted, not_found))
    }

    async fn delete_by_filter(
        &self,
        filter: MetadataFilter,
    ) -> Result<DeleteDocumentsResult, DomainError> {
        let filter_sql = self.filter_to_sql(&filter, None);

        if filter_sql.is_empty() {
            return Ok(DeleteDocumentsResult::new(0, 0));
        }

        let query = format!(
            "DELETE FROM {} WHERE kb_id = '{}' AND {}",
            self.config.table_name,
            self.id.as_str(),
            filter_sql
        );

        tracing::debug!(query = %query, "Executing delete by filter");

        let result = sqlx::query(&query).execute(&self.pool).await.map_err(|e| {
            tracing::error!(error = %e, query = %query, "Failed to delete by filter");
            DomainError::knowledge_base(format!("Failed to delete by filter: {}", e))
        })?;

        Ok(DeleteDocumentsResult::new(result.rows_affected() as usize, 0))
    }

    async fn get_document(&self, id: &str) -> Result<Option<SearchResult>, DomainError> {
        let query = format!(
            "SELECT id, content, metadata, source, embedding FROM {} WHERE kb_id = $1 AND id = $2",
            self.config.table_name
        );

        let result = sqlx::query(&query)
            .bind(self.id.as_str())
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| DomainError::knowledge_base(format!("Failed to get document: {}", e)))?;

        match result {
            Some(row) => {
                let id: String = row.get("id");
                let content: String = row.get("content");
                let metadata: serde_json::Value = row.get("metadata");
                let source: Option<String> = row.get("source");

                let metadata_map: HashMap<String, serde_json::Value> =
                    serde_json::from_value(metadata).unwrap_or_default();

                let mut result =
                    SearchResult::new(&id, &content, 1.0).with_all_metadata(metadata_map);

                if let Some(src) = source {
                    result = result.with_source(src);
                }

                Ok(Some(result))
            }
            None => Ok(None),
        }
    }

    async fn health_check(&self) -> Result<bool, DomainError> {
        let result = sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| DomainError::knowledge_base(format!("Health check failed: {}", e)))?;

        let _: i32 = result.get(0);
        Ok(true)
    }

    async fn document_count(&self) -> Result<usize, DomainError> {
        let query = format!(
            "SELECT COUNT(*) as count FROM {} WHERE kb_id = $1",
            self.config.table_name
        );

        let row = sqlx::query(&query)
            .bind(self.id.as_str())
            .fetch_one(&self.pool)
            .await
            .map_err(|e| DomainError::knowledge_base(format!("Failed to count documents: {}", e)))?;

        let count: i64 = row.get("count");
        Ok(count as usize)
    }

    async fn list_by_source(&self, source: &str) -> Result<Vec<SearchResult>, DomainError> {
        let query = format!(
            "SELECT id, content, metadata, source FROM {} WHERE kb_id = $1 AND source = $2 ORDER BY id",
            self.config.table_name
        );

        let rows = sqlx::query(&query)
            .bind(self.id.as_str())
            .bind(source)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                DomainError::knowledge_base(format!("Failed to list documents by source: {}", e))
            })?;

        let mut results = Vec::with_capacity(rows.len());

        for row in rows {
            let id: String = row.get("id");
            let content: String = row.get("content");
            let metadata: serde_json::Value = row.get("metadata");
            let source: Option<String> = row.get("source");

            let metadata_map: HashMap<String, serde_json::Value> =
                serde_json::from_value(metadata).unwrap_or_default();

            let mut result = SearchResult::new(&id, &content, 1.0).with_all_metadata(metadata_map);

            if let Some(src) = source {
                result = result.with_source(src);
            }

            results.push(result);
        }

        Ok(results)
    }

    async fn delete_by_source(&self, source: &str) -> Result<DeleteDocumentsResult, DomainError> {
        let query = format!(
            "DELETE FROM {} WHERE kb_id = $1 AND source = $2",
            self.config.table_name
        );

        let result = sqlx::query(&query)
            .bind(self.id.as_str())
            .bind(source)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                DomainError::knowledge_base(format!("Failed to delete documents by source: {}", e))
            })?;

        Ok(DeleteDocumentsResult::new(result.rows_affected() as usize, 0))
    }

    async fn list_sources(&self) -> Result<Vec<SourceInfo>, DomainError> {
        let query = format!(
            "SELECT source, COUNT(*) as doc_count FROM {} WHERE kb_id = $1 AND source IS NOT NULL GROUP BY source ORDER BY source",
            self.config.table_name
        );

        let rows = sqlx::query(&query)
            .bind(self.id.as_str())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                DomainError::knowledge_base(format!("Failed to list sources: {}", e))
            })?;

        let mut sources = Vec::with_capacity(rows.len());

        for row in rows {
            let source: String = row.get("source");
            let doc_count: i64 = row.get("doc_count");

            sources.push(SourceInfo {
                source,
                document_count: doc_count as usize,
            });
        }

        Ok(sources)
    }

    async fn ensure_schema(&self) -> Result<(), DomainError> {
        self.ensure_table().await
    }

    // ========================================================================
    // New document-based methods (for the new schema)
    // ========================================================================

    async fn create_document(
        &self,
        request: CreateDocumentRequest,
    ) -> Result<KnowledgeBaseDocument, DomainError> {
        // Insert document
        let doc_id = Uuid::new_v4();
        let chunk_count = request.chunks.len() as i32;
        let original_size = request.original_content.len() as i64;
        let metadata_json = serde_json::to_value(&request.metadata).unwrap_or_default();

        sqlx::query(
            r#"
            INSERT INTO knowledge_base_documents
            (id, kb_id, title, description, source_filename, content_type, original_size_bytes, chunk_count, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(doc_id)
        .bind(self.id.as_str())
        .bind(&request.title)
        .bind(&request.description)
        .bind(&request.source_filename)
        .bind(&request.content_type)
        .bind(original_size)
        .bind(chunk_count)
        .bind(&metadata_json)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::knowledge_base(format!("Failed to insert document: {}", e)))?;

        // Insert chunks
        for chunk in &request.chunks {
            let chunk_id = Uuid::new_v4();
            let embedding_str = format!("[{}]", chunk.embedding.iter().map(|f| f.to_string()).collect::<Vec<_>>().join(","));
            let chunk_metadata = serde_json::to_value(&chunk.metadata).unwrap_or_default();

            sqlx::query(
                r#"
                INSERT INTO knowledge_base_document_chunks
                (id, document_id, kb_id, chunk_index, content, embedding, token_count, metadata)
                VALUES ($1, $2, $3, $4, $5, $6::vector, $7, $8)
                "#,
            )
            .bind(chunk_id)
            .bind(doc_id)
            .bind(self.id.as_str())
            .bind(chunk.chunk_index)
            .bind(&chunk.content)
            .bind(&embedding_str)
            .bind(chunk.token_count)
            .bind(&chunk_metadata)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::knowledge_base(format!("Failed to insert chunk: {}", e)))?;
        }

        // Return the created document
        Ok(KnowledgeBaseDocument::new(self.id.as_str())
            .with_id(doc_id)
            .with_chunk_count(chunk_count)
            .with_original_size(original_size)
            .with_metadata(request.metadata))
    }

    async fn get_document_by_id(&self, id: Uuid) -> Result<Option<KnowledgeBaseDocument>, DomainError> {
        let row = sqlx::query(
            r#"
            SELECT id, kb_id, title, description, source_filename, content_type,
                   original_size_bytes, chunk_count, metadata, disabled, created_at, updated_at
            FROM knowledge_base_documents
            WHERE id = $1 AND kb_id = $2
            "#,
        )
        .bind(id)
        .bind(self.id.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::knowledge_base(format!("Failed to get document: {}", e)))?;

        match row {
            Some(row) => {
                let metadata: serde_json::Value = row.get("metadata");
                let metadata_map: HashMap<String, serde_json::Value> =
                    serde_json::from_value(metadata).unwrap_or_default();

                Ok(Some(
                    KnowledgeBaseDocument::new(self.id.as_str())
                        .with_id(row.get("id"))
                        .with_chunk_count(row.get("chunk_count"))
                        .with_original_size(row.get::<Option<i64>, _>("original_size_bytes").unwrap_or(0))
                        .with_metadata(metadata_map)
                        .with_disabled(row.get("disabled"))
                        .with_timestamps(row.get("created_at"), row.get("updated_at")),
                ))
            }
            None => Ok(None),
        }
    }

    async fn list_documents(&self) -> Result<Vec<DocumentSummary>, DomainError> {
        let rows = sqlx::query(
            r#"
            SELECT id, title, source_filename, chunk_count, disabled, created_at
            FROM knowledge_base_documents
            WHERE kb_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(self.id.as_str())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::knowledge_base(format!("Failed to list documents: {}", e)))?;

        Ok(rows
            .into_iter()
            .map(|row| DocumentSummary {
                id: row.get("id"),
                title: row.get("title"),
                source_filename: row.get("source_filename"),
                chunk_count: row.get("chunk_count"),
                disabled: row.get("disabled"),
                created_at: row.get("created_at"),
            })
            .collect())
    }

    async fn get_document_chunks(&self, document_id: Uuid) -> Result<Vec<DocumentChunk>, DomainError> {
        let rows = sqlx::query(
            r#"
            SELECT id, document_id, kb_id, chunk_index, content, embedding::text, token_count, metadata, created_at
            FROM knowledge_base_document_chunks
            WHERE document_id = $1 AND kb_id = $2
            ORDER BY chunk_index
            "#,
        )
        .bind(document_id)
        .bind(self.id.as_str())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::knowledge_base(format!("Failed to get chunks: {}", e)))?;

        Ok(rows
            .into_iter()
            .map(|row| {
                let embedding_str: String = row.get("embedding");
                let embedding = parse_pgvector(&embedding_str).unwrap_or_default();
                let metadata: serde_json::Value = row.get("metadata");
                let metadata_map: HashMap<String, serde_json::Value> =
                    serde_json::from_value(metadata).unwrap_or_default();

                DocumentChunk::new(
                    row.get("document_id"),
                    self.id.as_str(),
                    row.get("chunk_index"),
                    row.get::<String, _>("content"),
                )
                .with_id(row.get("id"))
                .with_embedding(embedding)
                .with_token_count(row.get::<Option<i32>, _>("token_count").unwrap_or(0))
                .with_metadata(metadata_map)
                .with_created_at(row.get("created_at"))
            })
            .collect())
    }

    async fn delete_document_by_id(&self, id: Uuid) -> Result<bool, DomainError> {
        // Chunks are deleted via ON DELETE CASCADE
        let result = sqlx::query("DELETE FROM knowledge_base_documents WHERE id = $1 AND kb_id = $2")
            .bind(id)
            .bind(self.id.as_str())
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::knowledge_base(format!("Failed to delete document: {}", e)))?;

        Ok(result.rows_affected() > 0)
    }

    async fn disable_document(&self, id: Uuid) -> Result<bool, DomainError> {
        let result = sqlx::query(
            "UPDATE knowledge_base_documents SET disabled = true, updated_at = NOW() WHERE id = $1 AND kb_id = $2",
        )
        .bind(id)
        .bind(self.id.as_str())
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::knowledge_base(format!("Failed to disable document: {}", e)))?;

        Ok(result.rows_affected() > 0)
    }

    async fn enable_document(&self, id: Uuid) -> Result<bool, DomainError> {
        let result = sqlx::query(
            "UPDATE knowledge_base_documents SET disabled = false, updated_at = NOW() WHERE id = $1 AND kb_id = $2",
        )
        .bind(id)
        .bind(self.id.as_str())
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::knowledge_base(format!("Failed to enable document: {}", e)))?;

        Ok(result.rows_affected() > 0)
    }
}

/// Parse a pgvector string representation back to a Vec<f32>
fn parse_pgvector(s: &str) -> Result<Vec<f32>, DomainError> {
    let trimmed = s.trim_start_matches('[').trim_end_matches(']');
    let values: Result<Vec<f32>, _> = trimmed.split(',').map(|v| v.trim().parse::<f32>()).collect();
    values.map_err(|e| DomainError::knowledge_base(format!("Failed to parse vector: {}", e)))
}

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    /// Mock embedding provider for testing
    #[derive(Debug, Clone)]
    pub struct MockEmbeddingProvider {
        dimensions: u32,
        embeddings: Arc<RwLock<HashMap<String, Vec<f32>>>>,
    }

    impl MockEmbeddingProvider {
        pub fn new(dimensions: u32) -> Self {
            Self {
                dimensions,
                embeddings: Arc::new(RwLock::new(HashMap::new())),
            }
        }

        pub async fn set_embedding(&self, text: &str, embedding: Vec<f32>) {
            self.embeddings
                .write()
                .await
                .insert(text.to_string(), embedding);
        }
    }

    #[async_trait]
    impl EmbeddingProvider for MockEmbeddingProvider {
        async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, DomainError> {
            let embeddings = self.embeddings.read().await;
            let mut results = Vec::with_capacity(texts.len());

            for text in texts {
                let embedding = embeddings.get(&text).cloned().unwrap_or_else(|| {
                    // Generate deterministic mock embedding based on text hash
                    let hash = text.bytes().fold(0u32, |acc, b| acc.wrapping_add(b as u32));
                    (0..self.dimensions)
                        .map(|i| ((hash.wrapping_add(i)) % 100) as f32 / 100.0)
                        .collect()
                });
                results.push(embedding);
            }

            Ok(results)
        }

        fn dimensions(&self) -> u32 {
            self.dimensions
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pgvector_config() {
        let config = PgvectorConfig::new(1536)
            .with_table_name("my_vectors")
            .with_distance_metric(DistanceMetric::Euclidean);

        assert_eq!(config.dimensions, 1536);
        assert_eq!(config.table_name, "my_vectors");
        assert_eq!(config.distance_metric, DistanceMetric::Euclidean);
    }

    #[test]
    fn test_distance_metric_operators() {
        assert_eq!(DistanceMetric::Cosine.operator(), "<=>");
        assert_eq!(DistanceMetric::Euclidean.operator(), "<->");
        assert_eq!(DistanceMetric::InnerProduct.operator(), "<#>");
    }

    #[test]
    fn test_distance_to_similarity() {
        let cosine = DistanceMetric::Cosine;
        assert!((cosine.to_similarity(0.0) - 1.0).abs() < 0.001);
        assert!((cosine.to_similarity(1.0) - 0.0).abs() < 0.001);

        let euclidean = DistanceMetric::Euclidean;
        assert!((euclidean.to_similarity(0.0) - 1.0).abs() < 0.001);
        assert!(euclidean.to_similarity(1.0) > 0.0);
    }

    #[test]
    fn test_parse_pgvector() {
        let result = parse_pgvector("[0.1, 0.2, 0.3]").unwrap();
        assert_eq!(result.len(), 3);
        assert!((result[0] - 0.1).abs() < 0.001);
        assert!((result[1] - 0.2).abs() < 0.001);
        assert!((result[2] - 0.3).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_mock_embedding_provider() {
        let provider = mock::MockEmbeddingProvider::new(3);

        provider
            .set_embedding("test", vec![0.1, 0.2, 0.3])
            .await;

        let result = provider.embed(vec!["test".to_string()]).await.unwrap();
        assert_eq!(result[0], vec![0.1, 0.2, 0.3]);

        // Test default embedding generation
        let result = provider
            .embed(vec!["unknown".to_string()])
            .await
            .unwrap();
        assert_eq!(result[0].len(), 3);
    }
}
