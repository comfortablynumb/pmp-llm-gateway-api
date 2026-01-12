//! Knowledge Bases management admin endpoints

use std::collections::HashMap;

use axum::{
    extract::{Multipart, Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use tracing::debug;

use uuid::Uuid;

use crate::api::middleware::RequireAdmin;
use crate::api::state::AppState;
use crate::api::types::ApiError;
use crate::domain::ingestion::{ChunkingType, ParserType};
use crate::domain::knowledge_base::{KnowledgeBaseConfig, KnowledgeBaseType};
use crate::infrastructure::services::{
    CreateKnowledgeBaseRequest, IngestDocumentRequest, IngestDocumentV2Request,
    UpdateKnowledgeBaseRequest,
};

/// Knowledge base type info response
#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeBaseTypeInfo {
    pub kb_type: String,
    pub description: String,
}

/// List knowledge base types response
#[derive(Debug, Clone, Serialize)]
pub struct ListKnowledgeBaseTypesResponse {
    pub types: Vec<KnowledgeBaseTypeInfo>,
}

/// Request to create a new knowledge base
#[derive(Debug, Clone, Deserialize)]
pub struct CreateKnowledgeBaseApiRequest {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub kb_type: String,
    pub embedding_model: String,
    pub embedding_dimensions: u32,
    pub credential_id: String,
    pub default_top_k: Option<u32>,
    pub default_similarity_threshold: Option<f32>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

/// Request to update a knowledge base
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateKnowledgeBaseApiRequest {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub default_top_k: Option<u32>,
    pub default_similarity_threshold: Option<f32>,
    pub enabled: Option<bool>,
}

/// Knowledge base response
#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeBaseResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub kb_type: String,
    pub embedding_model: String,
    pub embedding_dimensions: u32,
    pub credential_id: Option<String>,
    pub default_top_k: u32,
    pub default_similarity_threshold: f32,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// List knowledge bases response
#[derive(Debug, Clone, Serialize)]
pub struct ListKnowledgeBasesResponse {
    pub knowledge_bases: Vec<KnowledgeBaseResponse>,
    pub total: usize,
}

fn kb_type_to_string(kb_type: &KnowledgeBaseType) -> String {
    match kb_type {
        KnowledgeBaseType::Pgvector => "pgvector".to_string(),
        KnowledgeBaseType::AwsKnowledgeBase => "aws_knowledge_base".to_string(),
        KnowledgeBaseType::Pinecone => "pinecone".to_string(),
        KnowledgeBaseType::Weaviate => "weaviate".to_string(),
        KnowledgeBaseType::Qdrant => "qdrant".to_string(),
    }
}

fn parse_kb_type(s: &str) -> Result<KnowledgeBaseType, ApiError> {
    match s.to_lowercase().as_str() {
        "pgvector" | "pg_vector" => Ok(KnowledgeBaseType::Pgvector),
        "aws_knowledge_base" | "aws-knowledge-base" | "awsknowledgebase" => {
            Ok(KnowledgeBaseType::AwsKnowledgeBase)
        }
        "pinecone" => Ok(KnowledgeBaseType::Pinecone),
        "weaviate" => Ok(KnowledgeBaseType::Weaviate),
        "qdrant" => Ok(KnowledgeBaseType::Qdrant),
        other => Err(ApiError::bad_request(format!(
            "Unknown knowledge base type: {}",
            other
        ))),
    }
}

impl From<&crate::domain::KnowledgeBase> for KnowledgeBaseResponse {
    fn from(kb: &crate::domain::KnowledgeBase) -> Self {
        let credential_id = kb
            .connection_config()
            .and_then(|cc| cc.get("credential_id").cloned());

        Self {
            id: kb.id().as_str().to_string(),
            name: kb.name().to_string(),
            description: kb.description().map(|s| s.to_string()),
            kb_type: kb_type_to_string(kb.kb_type()),
            embedding_model: kb.embedding().model.clone(),
            embedding_dimensions: kb.embedding().dimensions,
            credential_id,
            default_top_k: kb.config().default_top_k,
            default_similarity_threshold: kb.config().default_similarity_threshold,
            enabled: kb.is_enabled(),
            created_at: kb.created_at().to_rfc3339(),
            updated_at: kb.updated_at().to_rfc3339(),
        }
    }
}

/// GET /admin/knowledge-bases/types
/// Lists available knowledge base types
pub async fn list_knowledge_base_types(
    State(_state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
) -> Result<Json<ListKnowledgeBaseTypesResponse>, ApiError> {
    debug!("Admin listing knowledge base types");

    let types = vec![
        KnowledgeBaseTypeInfo {
            kb_type: kb_type_to_string(&KnowledgeBaseType::Pgvector),
            description: "PostgreSQL with pgvector extension".to_string(),
        },
        KnowledgeBaseTypeInfo {
            kb_type: kb_type_to_string(&KnowledgeBaseType::AwsKnowledgeBase),
            description: "AWS Bedrock Knowledge Base".to_string(),
        },
        KnowledgeBaseTypeInfo {
            kb_type: kb_type_to_string(&KnowledgeBaseType::Pinecone),
            description: "Pinecone vector database".to_string(),
        },
    ];

    Ok(Json(ListKnowledgeBaseTypesResponse { types }))
}

/// GET /admin/knowledge-bases
/// List all knowledge bases
pub async fn list_knowledge_bases(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
) -> Result<Json<ListKnowledgeBasesResponse>, ApiError> {
    debug!("Admin listing all knowledge bases");

    let knowledge_bases = state
        .knowledge_base_service
        .list()
        .await
        .map_err(ApiError::from)?;

    let kb_responses: Vec<KnowledgeBaseResponse> =
        knowledge_bases.iter().map(KnowledgeBaseResponse::from).collect();
    let total = kb_responses.len();

    Ok(Json(ListKnowledgeBasesResponse {
        knowledge_bases: kb_responses,
        total,
    }))
}

/// POST /admin/knowledge-bases
/// Create a new knowledge base
pub async fn create_knowledge_base(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Json(request): Json<CreateKnowledgeBaseApiRequest>,
) -> Result<Json<KnowledgeBaseResponse>, ApiError> {
    debug!(kb_id = %request.id, "Admin creating knowledge base");

    let kb_type = parse_kb_type(&request.kb_type)?;

    // Verify credential exists and is a KB credential type
    let credential = state
        .credential_service
        .get(&request.credential_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| {
            ApiError::bad_request(format!("Credential '{}' not found", request.credential_id))
        })?;

    // Validate credential type matches KB requirements
    let valid_kb_cred_types = ["pgvector", "aws_knowledge_base", "pinecone"];
    let cred_type_str = credential.credential_type().to_string();

    if !valid_kb_cred_types.contains(&cred_type_str.as_str()) {
        return Err(ApiError::bad_request(format!(
            "Credential '{}' is not a knowledge base credential (type: {})",
            request.credential_id, cred_type_str
        )));
    }

    let config = KnowledgeBaseConfig::new()
        .with_default_top_k(request.default_top_k.unwrap_or(10))
        .with_default_similarity_threshold(request.default_similarity_threshold.unwrap_or(0.7));

    let create_request = CreateKnowledgeBaseRequest {
        id: request.id,
        name: request.name,
        description: request.description,
        kb_type,
        embedding_model: request.embedding_model,
        embedding_dimensions: request.embedding_dimensions,
        credential_id: request.credential_id,
        config: Some(config),
        enabled: request.enabled,
    };

    let kb = state
        .knowledge_base_service
        .create(create_request)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(KnowledgeBaseResponse::from(&kb)))
}

/// GET /admin/knowledge-bases/:kb_id
/// Get a specific knowledge base
pub async fn get_knowledge_base(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(kb_id): Path<String>,
) -> Result<Json<KnowledgeBaseResponse>, ApiError> {
    debug!(kb_id = %kb_id, "Admin getting knowledge base");

    let kb = state
        .knowledge_base_service
        .get(&kb_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::not_found(format!("Knowledge base '{}' not found", kb_id)))?;

    Ok(Json(KnowledgeBaseResponse::from(&kb)))
}

/// PUT /admin/knowledge-bases/:kb_id
/// Update a knowledge base
pub async fn update_knowledge_base(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(kb_id): Path<String>,
    Json(request): Json<UpdateKnowledgeBaseApiRequest>,
) -> Result<Json<KnowledgeBaseResponse>, ApiError> {
    debug!(kb_id = %kb_id, "Admin updating knowledge base");

    // Build config if any config fields provided
    let config = if request.default_top_k.is_some() || request.default_similarity_threshold.is_some()
    {
        // Get existing KB to preserve config values
        let existing = state
            .knowledge_base_service
            .get(&kb_id)
            .await
            .map_err(ApiError::from)?
            .ok_or_else(|| ApiError::not_found(format!("Knowledge base '{}' not found", kb_id)))?;

        let mut config = existing.config().clone();

        if let Some(top_k) = request.default_top_k {
            config.default_top_k = top_k;
        }

        if let Some(threshold) = request.default_similarity_threshold {
            config.default_similarity_threshold = threshold;
        }

        Some(config)
    } else {
        None
    };

    let update_request = UpdateKnowledgeBaseRequest {
        name: request.name,
        description: request.description,
        config,
        enabled: request.enabled,
    };

    let kb = state
        .knowledge_base_service
        .update(&kb_id, update_request)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(KnowledgeBaseResponse::from(&kb)))
}

/// DELETE /admin/knowledge-bases/:kb_id
/// Delete a knowledge base
pub async fn delete_knowledge_base(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(kb_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    debug!(kb_id = %kb_id, "Admin deleting knowledge base");

    state
        .knowledge_base_service
        .delete(&kb_id)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(serde_json::json!({
        "deleted": true,
        "id": kb_id
    })))
}

// ============================================================================
// Document Ingestion Endpoints
// ============================================================================

/// Request to ingest a document into a knowledge base
#[derive(Debug, Clone, Deserialize)]
pub struct IngestDocumentApiRequest {
    pub content: String,
    #[serde(default)]
    pub filename: Option<String>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub source_id: Option<String>,
    #[serde(default)]
    pub parser_type: Option<String>,
    #[serde(default)]
    pub chunking_type: Option<String>,
    #[serde(default)]
    pub chunk_size: Option<usize>,
    #[serde(default)]
    pub chunk_overlap: Option<usize>,
}

/// Response from document ingestion
#[derive(Debug, Clone, Serialize)]
pub struct IngestDocumentResponse {
    pub document_id: String,
    pub chunks_created: usize,
    pub chunks_failed: usize,
    pub errors: Vec<String>,
}

/// A stored document
#[derive(Debug, Clone, Serialize)]
pub struct DocumentResponse {
    pub id: String,
    pub content: String,
    pub metadata: HashMap<String, serde_json::Value>,
    pub source: Option<String>,
    pub chunk_index: usize,
    pub total_chunks: usize,
}

/// List documents response
#[derive(Debug, Clone, Serialize)]
pub struct ListDocumentsResponse {
    pub documents: Vec<DocumentResponse>,
    pub total: usize,
}

/// A source entry (grouped documents)
#[derive(Debug, Clone, Serialize)]
pub struct SourceResponse {
    pub source: String,
    pub document_count: usize,
}

/// List sources response
#[derive(Debug, Clone, Serialize)]
pub struct ListSourcesResponse {
    pub sources: Vec<SourceResponse>,
    pub total: usize,
}

fn parse_parser_type(s: &str) -> Result<ParserType, ApiError> {
    match s.to_lowercase().as_str() {
        "plain_text" | "plaintext" | "text" | "txt" => Ok(ParserType::PlainText),
        "markdown" | "md" => Ok(ParserType::Markdown),
        "html" => Ok(ParserType::Html),
        "json" => Ok(ParserType::Json),
        other => Err(ApiError::bad_request(format!(
            "Unknown parser type: {}",
            other
        ))),
    }
}

fn parse_chunking_type(s: &str) -> Result<ChunkingType, ApiError> {
    match s.to_lowercase().as_str() {
        "fixed_size" | "fixedsize" | "fixed" => Ok(ChunkingType::FixedSize),
        "sentence" | "sentences" => Ok(ChunkingType::Sentence),
        "paragraph" | "paragraphs" => Ok(ChunkingType::Paragraph),
        "recursive" => Ok(ChunkingType::Recursive),
        other => Err(ApiError::bad_request(format!(
            "Unknown chunking type: {}",
            other
        ))),
    }
}

/// Response for batch file upload
#[derive(Debug, Clone, Serialize)]
pub struct BatchIngestResponse {
    pub total: usize,
    pub log_ids: Vec<String>,
    pub message: String,
}

/// POST /admin/knowledge-bases/:kb_id/documents/upload
/// Batch ingest files via multipart form upload
pub async fn ingest_files_batch(
    State(state): State<AppState>,
    RequireAdmin(admin_claims): RequireAdmin,
    Path(kb_id): Path<String>,
    mut multipart: Multipart,
) -> Result<Json<BatchIngestResponse>, ApiError> {
    debug!(kb_id = %kb_id, "Admin batch ingesting files via multipart upload");

    // Verify knowledge base exists
    let kb_exists = state
        .knowledge_base_service
        .exists(&kb_id)
        .await
        .map_err(ApiError::from)?;

    if !kb_exists {
        return Err(ApiError::not_found(format!(
            "Knowledge base '{}' not found",
            kb_id
        )));
    }

    // Create executor from admin auth
    let executor = match &admin_claims {
        crate::api::middleware::AdminAuth::User(user) => {
            crate::domain::Executor::from_user(user.id().as_str())
        }
        crate::api::middleware::AdminAuth::ApiKey(api_key) => {
            crate::domain::Executor::from_api_key(api_key.id().as_str())
        }
    };

    // Collect files from multipart form
    let mut files: Vec<(String, String)> = Vec::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::bad_request(format!("Failed to read multipart field: {}", e)))?
    {
        let filename = field
            .file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("file-{}", uuid::Uuid::new_v4()));

        let content = field
            .text()
            .await
            .map_err(|e| ApiError::bad_request(format!("Failed to read file '{}': {}", filename, e)))?;

        if !content.is_empty() {
            files.push((filename, content));
        }
    }

    if files.is_empty() {
        return Err(ApiError::bad_request("No files provided"));
    }

    let mut log_ids = Vec::with_capacity(files.len());

    // Process each file
    for (filename, content) in files {
        // Create execution log
        let input_json = serde_json::json!({
            "kb_id": kb_id,
            "source": filename,
            "content_length": content.len(),
            "parser_type": "auto",
        });

        let mut log = state
            .execution_log_service
            .record_pending_ingestion(&kb_id, &filename, executor.clone(), input_json)
            .await
            .map_err(ApiError::from)?;

        log_ids.push(log.id().as_str().to_string());

        // Clone for async task
        let ingestion_service = state.ingestion_service.clone();
        let execution_log_service = state.execution_log_service.clone();
        let kb_id_clone = kb_id.clone();
        let filename_clone = filename.clone();

        // Spawn async ingestion task
        tokio::spawn(Box::pin(async move {
            let start = std::time::Instant::now();

            // Mark as in progress
            log.set_in_progress();
            let _ = execution_log_service.update(&log).await;

            // Build ingestion request - use filename for auto-detection of parser type
            let ingest_request = IngestDocumentRequest::new(content)
                .with_filename(filename_clone.clone())
                .with_source_id(filename_clone);

            // Perform ingestion
            let execution_time_ms = start.elapsed().as_millis() as u64;

            match ingestion_service.ingest(&kb_id_clone, ingest_request).await {
                Ok(result) => {
                    let output = serde_json::json!({
                        "document_id": result.document_id,
                        "chunks_created": result.chunks_created,
                        "chunks_failed": result.chunks_failed,
                        "errors": result.errors.iter().map(|e| &e.message).collect::<Vec<_>>(),
                    });
                    log.set_success(execution_time_ms, Some(output));
                }
                Err(e) => {
                    log.set_failed(execution_time_ms, e.to_string());
                }
            }

            let _ = execution_log_service.update(&log).await;
        }));
    }

    let total = log_ids.len();

    Ok(Json(BatchIngestResponse {
        total,
        log_ids,
        message: format!("{} file(s) queued for ingestion. Check execution logs for progress.", total),
    }))
}

/// Ingestion operation info
#[derive(Debug, Clone, Serialize)]
pub struct IngestionOperationResponse {
    pub id: String,
    pub source_name: Option<String>,
    pub status: String,
    pub error: Option<String>,
    pub created_at: String,
    pub execution_time_ms: u64,
}

/// List ingestion operations response
#[derive(Debug, Clone, Serialize)]
pub struct ListIngestionOperationsResponse {
    pub operations: Vec<IngestionOperationResponse>,
    pub total: usize,
}

/// GET /admin/knowledge-bases/:kb_id/ingestions
/// List ingestion operations for a knowledge base
pub async fn list_ingestion_operations(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(kb_id): Path<String>,
) -> Result<Json<ListIngestionOperationsResponse>, ApiError> {
    debug!(kb_id = %kb_id, "Admin listing ingestion operations");

    // Verify knowledge base exists
    let kb_exists = state
        .knowledge_base_service
        .exists(&kb_id)
        .await
        .map_err(ApiError::from)?;

    if !kb_exists {
        return Err(ApiError::not_found(format!(
            "Knowledge base '{}' not found",
            kb_id
        )));
    }

    // Query execution logs for this KB's ingestion operations
    let query = crate::domain::ExecutionLogQuery::new()
        .with_execution_type(crate::domain::ExecutionType::Ingestion)
        .with_resource_id(&kb_id)
        .with_limit(100);

    let logs = state
        .execution_log_service
        .list(&query)
        .await
        .map_err(ApiError::from)?;

    let operations: Vec<IngestionOperationResponse> = logs
        .into_iter()
        .map(|log| IngestionOperationResponse {
            id: log.id().as_str().to_string(),
            source_name: log.resource_name().map(|s| s.to_string()),
            status: log.status().as_str().to_string(),
            error: log.error().map(|s| s.to_string()),
            created_at: log.created_at().to_rfc3339(),
            execution_time_ms: log.execution_time_ms(),
        })
        .collect();

    let total = operations.len();

    Ok(Json(ListIngestionOperationsResponse { operations, total }))
}

/// POST /admin/knowledge-bases/:kb_id/schema
/// Ensure the knowledge base schema exists (create tables/indexes)
pub async fn ensure_schema(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(kb_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    debug!(kb_id = %kb_id, "Admin ensuring schema for knowledge base");

    // Verify knowledge base exists
    let kb_exists = state
        .knowledge_base_service
        .exists(&kb_id)
        .await
        .map_err(ApiError::from)?;

    if !kb_exists {
        return Err(ApiError::not_found(format!(
            "Knowledge base '{}' not found",
            kb_id
        )));
    }

    state
        .ingestion_service
        .ensure_schema(&kb_id)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(serde_json::json!({
        "kb_id": kb_id,
        "schema_initialized": true
    })))
}

// ============================================================================
// New Schema Endpoints (document/chunk separation)
// ============================================================================

/// Request to ingest a document using the new schema
#[derive(Debug, Clone, Deserialize)]
pub struct IngestDocumentV2ApiRequest {
    pub content: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub filename: Option<String>,
    #[serde(default)]
    pub content_type: Option<String>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub parser_type: Option<String>,
    #[serde(default)]
    pub chunking_type: Option<String>,
    #[serde(default)]
    pub chunk_size: Option<usize>,
    #[serde(default)]
    pub chunk_overlap: Option<usize>,
}

/// Response for document ingestion (new schema)
#[derive(Debug, Clone, Serialize)]
pub struct DocumentV2Response {
    pub id: String,
    pub kb_id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub source_filename: Option<String>,
    pub content_type: Option<String>,
    pub chunk_count: i32,
    pub disabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Document summary response
#[derive(Debug, Clone, Serialize)]
pub struct DocumentSummaryResponse {
    pub id: String,
    pub title: Option<String>,
    pub source_filename: Option<String>,
    pub chunk_count: i32,
    pub disabled: bool,
    pub created_at: String,
}

/// List documents response (new schema)
#[derive(Debug, Clone, Serialize)]
pub struct ListDocumentsV2Response {
    pub documents: Vec<DocumentSummaryResponse>,
    pub total: usize,
}

/// Document chunk response
#[derive(Debug, Clone, Serialize)]
pub struct DocumentChunkResponse {
    pub id: String,
    pub document_id: String,
    pub chunk_index: i32,
    pub content: String,
    pub token_count: Option<i32>,
    pub created_at: String,
}

/// List chunks response
#[derive(Debug, Clone, Serialize)]
pub struct ListChunksResponse {
    pub chunks: Vec<DocumentChunkResponse>,
    pub total: usize,
}

/// POST /admin/knowledge-bases/:kb_id/documents
/// Ingest a document into a knowledge base
pub async fn ingest_document(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(kb_id): Path<String>,
    Json(request): Json<IngestDocumentV2ApiRequest>,
) -> Result<Json<DocumentV2Response>, ApiError> {
    debug!(kb_id = %kb_id, "Admin ingesting document into knowledge base");

    // Verify knowledge base exists
    let kb_exists = state
        .knowledge_base_service
        .exists(&kb_id)
        .await
        .map_err(ApiError::from)?;

    if !kb_exists {
        return Err(ApiError::not_found(format!(
            "Knowledge base '{}' not found",
            kb_id
        )));
    }

    // Parse optional types
    let parser_type = if let Some(pt) = &request.parser_type {
        Some(parse_parser_type(pt)?)
    } else {
        None
    };

    let chunking_type = if let Some(ct) = &request.chunking_type {
        Some(parse_chunking_type(ct)?)
    } else {
        None
    };

    // Build ingestion request
    let mut ingest_request = IngestDocumentV2Request::new(request.content);

    if let Some(title) = request.title {
        ingest_request = ingest_request.with_title(title);
    }

    if let Some(description) = request.description {
        ingest_request = ingest_request.with_description(description);
    }

    if let Some(filename) = request.filename {
        ingest_request = ingest_request.with_filename(filename);
    }

    if let Some(content_type) = request.content_type {
        ingest_request = ingest_request.with_content_type(content_type);
    }

    for (key, value) in request.metadata {
        ingest_request = ingest_request.with_metadata(key, value);
    }

    ingest_request.parser_type = parser_type;
    ingest_request.chunking_type = chunking_type;
    ingest_request.chunk_size = request.chunk_size;
    ingest_request.chunk_overlap = request.chunk_overlap;

    // Perform ingestion
    let document = state
        .ingestion_service
        .ingest_document(&kb_id, ingest_request)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(DocumentV2Response {
        id: document.id().to_string(),
        kb_id: document.kb_id().to_string(),
        title: document.title().map(|s| s.to_string()),
        description: document.description().map(|s| s.to_string()),
        source_filename: document.source_filename().map(|s| s.to_string()),
        content_type: document.content_type().map(|s| s.to_string()),
        chunk_count: document.chunk_count(),
        disabled: document.is_disabled(),
        created_at: document.created_at().to_rfc3339(),
        updated_at: document.updated_at().to_rfc3339(),
    }))
}

/// GET /admin/knowledge-bases/:kb_id/documents
/// List all documents in a knowledge base
pub async fn list_documents(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(kb_id): Path<String>,
) -> Result<Json<ListDocumentsV2Response>, ApiError> {
    debug!(kb_id = %kb_id, "Admin listing documents in knowledge base");

    // Verify knowledge base exists
    let kb_exists = state
        .knowledge_base_service
        .exists(&kb_id)
        .await
        .map_err(ApiError::from)?;

    if !kb_exists {
        return Err(ApiError::not_found(format!(
            "Knowledge base '{}' not found",
            kb_id
        )));
    }

    let documents = state
        .ingestion_service
        .list_documents_v2(&kb_id)
        .await
        .map_err(ApiError::from)?;

    let doc_responses: Vec<DocumentSummaryResponse> = documents
        .into_iter()
        .map(|doc| DocumentSummaryResponse {
            id: doc.id.to_string(),
            title: doc.title,
            source_filename: doc.source_filename,
            chunk_count: doc.chunk_count,
            disabled: doc.disabled,
            created_at: doc.created_at.to_rfc3339(),
        })
        .collect();

    let total = doc_responses.len();

    Ok(Json(ListDocumentsV2Response {
        documents: doc_responses,
        total,
    }))
}

/// GET /admin/knowledge-bases/:kb_id/documents/:document_id
/// Get a document by ID
pub async fn get_document(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path((kb_id, document_id)): Path<(String, String)>,
) -> Result<Json<DocumentV2Response>, ApiError> {
    debug!(kb_id = %kb_id, document_id = %document_id, "Admin getting document");

    // Verify knowledge base exists
    let kb_exists = state
        .knowledge_base_service
        .exists(&kb_id)
        .await
        .map_err(ApiError::from)?;

    if !kb_exists {
        return Err(ApiError::not_found(format!(
            "Knowledge base '{}' not found",
            kb_id
        )));
    }

    let doc_uuid = Uuid::parse_str(&document_id)
        .map_err(|_| ApiError::bad_request(format!("Invalid document ID: {}", document_id)))?;

    let document = state
        .ingestion_service
        .get_document_v2(&kb_id, doc_uuid)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::not_found(format!("Document '{}' not found", document_id)))?;

    Ok(Json(DocumentV2Response {
        id: document.id().to_string(),
        kb_id: document.kb_id().to_string(),
        title: document.title().map(|s| s.to_string()),
        description: document.description().map(|s| s.to_string()),
        source_filename: document.source_filename().map(|s| s.to_string()),
        content_type: document.content_type().map(|s| s.to_string()),
        chunk_count: document.chunk_count(),
        disabled: document.is_disabled(),
        created_at: document.created_at().to_rfc3339(),
        updated_at: document.updated_at().to_rfc3339(),
    }))
}

/// GET /admin/knowledge-bases/:kb_id/documents/:document_id/chunks
/// Get chunks for a document
pub async fn get_document_chunks(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path((kb_id, document_id)): Path<(String, String)>,
) -> Result<Json<ListChunksResponse>, ApiError> {
    debug!(kb_id = %kb_id, document_id = %document_id, "Admin getting document chunks");

    // Verify knowledge base exists
    let kb_exists = state
        .knowledge_base_service
        .exists(&kb_id)
        .await
        .map_err(ApiError::from)?;

    if !kb_exists {
        return Err(ApiError::not_found(format!(
            "Knowledge base '{}' not found",
            kb_id
        )));
    }

    let doc_uuid = Uuid::parse_str(&document_id)
        .map_err(|_| ApiError::bad_request(format!("Invalid document ID: {}", document_id)))?;

    let chunks = state
        .ingestion_service
        .get_document_chunks(&kb_id, doc_uuid)
        .await
        .map_err(ApiError::from)?;

    let chunk_responses: Vec<DocumentChunkResponse> = chunks
        .into_iter()
        .map(|chunk| DocumentChunkResponse {
            id: chunk.id().to_string(),
            document_id: chunk.document_id().to_string(),
            chunk_index: chunk.chunk_index(),
            content: chunk.content().to_string(),
            token_count: chunk.token_count(),
            created_at: chunk.created_at().to_rfc3339(),
        })
        .collect();

    let total = chunk_responses.len();

    Ok(Json(ListChunksResponse {
        chunks: chunk_responses,
        total,
    }))
}

/// DELETE /admin/knowledge-bases/:kb_id/documents/:document_id
/// Delete a document and its chunks
pub async fn delete_document(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path((kb_id, document_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    debug!(kb_id = %kb_id, document_id = %document_id, "Admin deleting document");

    // Verify knowledge base exists
    let kb_exists = state
        .knowledge_base_service
        .exists(&kb_id)
        .await
        .map_err(ApiError::from)?;

    if !kb_exists {
        return Err(ApiError::not_found(format!(
            "Knowledge base '{}' not found",
            kb_id
        )));
    }

    let doc_uuid = Uuid::parse_str(&document_id)
        .map_err(|_| ApiError::bad_request(format!("Invalid document ID: {}", document_id)))?;

    let deleted = state
        .ingestion_service
        .delete_document_v2(&kb_id, doc_uuid)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(serde_json::json!({
        "deleted": deleted,
        "kb_id": kb_id,
        "document_id": document_id
    })))
}

/// POST /admin/knowledge-bases/:kb_id/documents/:document_id/disable
/// Disable a document (soft delete - excludes from search)
pub async fn disable_document(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path((kb_id, document_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    debug!(kb_id = %kb_id, document_id = %document_id, "Admin disabling document");

    // Verify knowledge base exists
    let kb_exists = state
        .knowledge_base_service
        .exists(&kb_id)
        .await
        .map_err(ApiError::from)?;

    if !kb_exists {
        return Err(ApiError::not_found(format!(
            "Knowledge base '{}' not found",
            kb_id
        )));
    }

    let doc_uuid = Uuid::parse_str(&document_id)
        .map_err(|_| ApiError::bad_request(format!("Invalid document ID: {}", document_id)))?;

    let disabled = state
        .ingestion_service
        .disable_document(&kb_id, doc_uuid)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(serde_json::json!({
        "disabled": disabled,
        "kb_id": kb_id,
        "document_id": document_id
    })))
}

/// POST /admin/knowledge-bases/:kb_id/documents/:document_id/enable
/// Enable a previously disabled document
pub async fn enable_document(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path((kb_id, document_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    debug!(kb_id = %kb_id, document_id = %document_id, "Admin enabling document");

    // Verify knowledge base exists
    let kb_exists = state
        .knowledge_base_service
        .exists(&kb_id)
        .await
        .map_err(ApiError::from)?;

    if !kb_exists {
        return Err(ApiError::not_found(format!(
            "Knowledge base '{}' not found",
            kb_id
        )));
    }

    let doc_uuid = Uuid::parse_str(&document_id)
        .map_err(|_| ApiError::bad_request(format!("Invalid document ID: {}", document_id)))?;

    let enabled = state
        .ingestion_service
        .enable_document(&kb_id, doc_uuid)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(serde_json::json!({
        "enabled": enabled,
        "kb_id": kb_id,
        "document_id": document_id
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kb_type_to_string() {
        assert_eq!(kb_type_to_string(&KnowledgeBaseType::Pgvector), "pgvector");
        assert_eq!(
            kb_type_to_string(&KnowledgeBaseType::AwsKnowledgeBase),
            "aws_knowledge_base"
        );
        assert_eq!(kb_type_to_string(&KnowledgeBaseType::Pinecone), "pinecone");
        assert_eq!(kb_type_to_string(&KnowledgeBaseType::Weaviate), "weaviate");
        assert_eq!(kb_type_to_string(&KnowledgeBaseType::Qdrant), "qdrant");
    }

    #[test]
    fn test_parse_kb_type_pgvector() {
        assert!(matches!(
            parse_kb_type("pgvector").unwrap(),
            KnowledgeBaseType::Pgvector
        ));
        assert!(matches!(
            parse_kb_type("pg_vector").unwrap(),
            KnowledgeBaseType::Pgvector
        ));
    }

    #[test]
    fn test_parse_kb_type_aws() {
        assert!(matches!(
            parse_kb_type("aws_knowledge_base").unwrap(),
            KnowledgeBaseType::AwsKnowledgeBase
        ));
        assert!(matches!(
            parse_kb_type("aws-knowledge-base").unwrap(),
            KnowledgeBaseType::AwsKnowledgeBase
        ));
        assert!(matches!(
            parse_kb_type("awsknowledgebase").unwrap(),
            KnowledgeBaseType::AwsKnowledgeBase
        ));
    }

    #[test]
    fn test_parse_kb_type_pinecone() {
        assert!(matches!(
            parse_kb_type("pinecone").unwrap(),
            KnowledgeBaseType::Pinecone
        ));
    }

    #[test]
    fn test_parse_kb_type_weaviate() {
        assert!(matches!(
            parse_kb_type("weaviate").unwrap(),
            KnowledgeBaseType::Weaviate
        ));
    }

    #[test]
    fn test_parse_kb_type_qdrant() {
        assert!(matches!(
            parse_kb_type("qdrant").unwrap(),
            KnowledgeBaseType::Qdrant
        ));
    }

    #[test]
    fn test_parse_kb_type_invalid() {
        assert!(parse_kb_type("unknown").is_err());
    }

    #[test]
    fn test_default_enabled() {
        assert!(default_enabled());
    }

    #[test]
    fn test_parse_parser_type_plain_text() {
        assert!(matches!(
            parse_parser_type("plain_text").unwrap(),
            ParserType::PlainText
        ));
        assert!(matches!(
            parse_parser_type("plaintext").unwrap(),
            ParserType::PlainText
        ));
        assert!(matches!(
            parse_parser_type("text").unwrap(),
            ParserType::PlainText
        ));
        assert!(matches!(
            parse_parser_type("txt").unwrap(),
            ParserType::PlainText
        ));
    }

    #[test]
    fn test_parse_parser_type_markdown() {
        assert!(matches!(
            parse_parser_type("markdown").unwrap(),
            ParserType::Markdown
        ));
        assert!(matches!(
            parse_parser_type("md").unwrap(),
            ParserType::Markdown
        ));
    }

    #[test]
    fn test_parse_parser_type_html() {
        assert!(matches!(
            parse_parser_type("html").unwrap(),
            ParserType::Html
        ));
    }

    #[test]
    fn test_parse_parser_type_json() {
        assert!(matches!(
            parse_parser_type("json").unwrap(),
            ParserType::Json
        ));
    }

    #[test]
    fn test_parse_parser_type_invalid() {
        assert!(parse_parser_type("unknown").is_err());
    }

    #[test]
    fn test_parse_chunking_type_fixed_size() {
        assert!(matches!(
            parse_chunking_type("fixed_size").unwrap(),
            ChunkingType::FixedSize
        ));
        assert!(matches!(
            parse_chunking_type("fixedsize").unwrap(),
            ChunkingType::FixedSize
        ));
        assert!(matches!(
            parse_chunking_type("fixed").unwrap(),
            ChunkingType::FixedSize
        ));
    }

    #[test]
    fn test_parse_chunking_type_sentence() {
        assert!(matches!(
            parse_chunking_type("sentence").unwrap(),
            ChunkingType::Sentence
        ));
        assert!(matches!(
            parse_chunking_type("sentences").unwrap(),
            ChunkingType::Sentence
        ));
    }

    #[test]
    fn test_parse_chunking_type_paragraph() {
        assert!(matches!(
            parse_chunking_type("paragraph").unwrap(),
            ChunkingType::Paragraph
        ));
        assert!(matches!(
            parse_chunking_type("paragraphs").unwrap(),
            ChunkingType::Paragraph
        ));
    }

    #[test]
    fn test_parse_chunking_type_recursive() {
        assert!(matches!(
            parse_chunking_type("recursive").unwrap(),
            ChunkingType::Recursive
        ));
    }

    #[test]
    fn test_parse_chunking_type_invalid() {
        assert!(parse_chunking_type("unknown").is_err());
    }

    #[test]
    fn test_knowledge_base_type_info_serialization() {
        let info = KnowledgeBaseTypeInfo {
            kb_type: "pgvector".to_string(),
            description: "PostgreSQL with pgvector extension".to_string(),
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"kb_type\":\"pgvector\""));
        assert!(json.contains("\"description\":\"PostgreSQL with pgvector extension\""));
    }

    #[test]
    fn test_list_knowledge_base_types_response_serialization() {
        let response = ListKnowledgeBaseTypesResponse {
            types: vec![
                KnowledgeBaseTypeInfo {
                    kb_type: "pgvector".to_string(),
                    description: "Test".to_string(),
                },
            ],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"types\":["));
        assert!(json.contains("\"pgvector\""));
    }

    #[test]
    fn test_create_knowledge_base_api_request_deserialization() {
        let json = r#"{
            "id": "kb-001",
            "name": "Test KB",
            "description": "A test knowledge base",
            "kb_type": "pgvector",
            "embedding_model": "text-embedding-ada-002",
            "embedding_dimensions": 1536,
            "credential_id": "cred-001",
            "default_top_k": 10,
            "default_similarity_threshold": 0.8,
            "enabled": true
        }"#;

        let request: CreateKnowledgeBaseApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.id, "kb-001");
        assert_eq!(request.name, "Test KB");
        assert_eq!(request.description, Some("A test knowledge base".to_string()));
        assert_eq!(request.kb_type, "pgvector");
        assert_eq!(request.embedding_model, "text-embedding-ada-002");
        assert_eq!(request.embedding_dimensions, 1536);
        assert_eq!(request.credential_id, "cred-001");
        assert_eq!(request.default_top_k, Some(10));
        assert_eq!(request.default_similarity_threshold, Some(0.8));
        assert!(request.enabled);
    }

    #[test]
    fn test_create_knowledge_base_api_request_minimal() {
        let json = r#"{
            "id": "kb-002",
            "name": "Minimal KB",
            "kb_type": "pgvector",
            "embedding_model": "text-embedding-ada-002",
            "embedding_dimensions": 1536,
            "credential_id": "cred-001"
        }"#;

        let request: CreateKnowledgeBaseApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.id, "kb-002");
        assert!(request.description.is_none());
        assert!(request.default_top_k.is_none());
        assert!(request.default_similarity_threshold.is_none());
        assert!(request.enabled);  // default_enabled returns true
    }

    #[test]
    fn test_update_knowledge_base_api_request_deserialization() {
        let json = r#"{
            "name": "Updated Name",
            "default_top_k": 20,
            "enabled": false
        }"#;

        let request: UpdateKnowledgeBaseApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.name, Some("Updated Name".to_string()));
        assert_eq!(request.default_top_k, Some(20));
        assert_eq!(request.enabled, Some(false));
        assert!(request.description.is_none());
    }

    #[test]
    fn test_knowledge_base_response_serialization() {
        let response = KnowledgeBaseResponse {
            id: "kb-001".to_string(),
            name: "Test KB".to_string(),
            description: Some("A description".to_string()),
            kb_type: "pgvector".to_string(),
            embedding_model: "text-embedding-ada-002".to_string(),
            embedding_dimensions: 1536,
            credential_id: Some("cred-001".to_string()),
            default_top_k: 10,
            default_similarity_threshold: 0.7,
            enabled: true,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"id\":\"kb-001\""));
        assert!(json.contains("\"name\":\"Test KB\""));
        assert!(json.contains("\"kb_type\":\"pgvector\""));
        assert!(json.contains("\"embedding_dimensions\":1536"));
        assert!(json.contains("\"default_top_k\":10"));
    }

    #[test]
    fn test_list_knowledge_bases_response_serialization() {
        let response = ListKnowledgeBasesResponse {
            knowledge_bases: vec![],
            total: 5,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"knowledge_bases\":[]"));
        assert!(json.contains("\"total\":5"));
    }

    #[test]
    fn test_ingest_document_api_request_deserialization() {
        let json = r#"{
            "content": "This is document content",
            "filename": "test.txt",
            "metadata": {"author": "test"},
            "source_id": "source-001",
            "parser_type": "plain_text",
            "chunking_type": "fixed_size",
            "chunk_size": 500,
            "chunk_overlap": 50
        }"#;

        let request: IngestDocumentApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.content, "This is document content");
        assert_eq!(request.filename, Some("test.txt".to_string()));
        assert_eq!(request.source_id, Some("source-001".to_string()));
        assert_eq!(request.parser_type, Some("plain_text".to_string()));
        assert_eq!(request.chunking_type, Some("fixed_size".to_string()));
        assert_eq!(request.chunk_size, Some(500));
        assert_eq!(request.chunk_overlap, Some(50));
    }

    #[test]
    fn test_ingest_document_api_request_minimal() {
        let json = r#"{"content": "Minimal content"}"#;

        let request: IngestDocumentApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.content, "Minimal content");
        assert!(request.filename.is_none());
        assert!(request.metadata.is_empty());
    }

    #[test]
    fn test_ingest_document_response_serialization() {
        let response = IngestDocumentResponse {
            document_id: "doc-001".to_string(),
            chunks_created: 10,
            chunks_failed: 1,
            errors: vec!["Chunk 5 failed".to_string()],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"document_id\":\"doc-001\""));
        assert!(json.contains("\"chunks_created\":10"));
        assert!(json.contains("\"chunks_failed\":1"));
        assert!(json.contains("Chunk 5 failed"));
    }

    #[test]
    fn test_document_response_serialization() {
        let mut metadata = HashMap::new();
        metadata.insert("author".to_string(), serde_json::json!("test"));

        let response = DocumentResponse {
            id: "chunk-001".to_string(),
            content: "Document content".to_string(),
            metadata,
            source: Some("source-001".to_string()),
            chunk_index: 0,
            total_chunks: 5,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"id\":\"chunk-001\""));
        assert!(json.contains("\"content\":\"Document content\""));
        assert!(json.contains("\"chunk_index\":0"));
        assert!(json.contains("\"total_chunks\":5"));
    }

    #[test]
    fn test_list_documents_response_serialization() {
        let response = ListDocumentsResponse {
            documents: vec![],
            total: 100,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"documents\":[]"));
        assert!(json.contains("\"total\":100"));
    }

    #[test]
    fn test_source_response_serialization() {
        let response = SourceResponse {
            source: "file.txt".to_string(),
            document_count: 10,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"source\":\"file.txt\""));
        assert!(json.contains("\"document_count\":10"));
    }

    #[test]
    fn test_list_sources_response_serialization() {
        let response = ListSourcesResponse {
            sources: vec![],
            total: 3,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"sources\":[]"));
        assert!(json.contains("\"total\":3"));
    }

    #[test]
    fn test_batch_ingest_response_serialization() {
        let response = BatchIngestResponse {
            total: 3,
            log_ids: vec!["log-1".to_string(), "log-2".to_string(), "log-3".to_string()],
            message: "3 ingestion(s) started".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"total\":3"));
        assert!(json.contains("\"log_ids\":["));
        assert!(json.contains("\"log-1\""));
    }

    #[test]
    fn test_ingestion_operation_response_serialization() {
        let response = IngestionOperationResponse {
            id: "op-001".to_string(),
            source_name: Some("document.txt".to_string()),
            status: "success".to_string(),
            error: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            execution_time_ms: 1500,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"id\":\"op-001\""));
        assert!(json.contains("\"source_name\":\"document.txt\""));
        assert!(json.contains("\"status\":\"success\""));
        assert!(json.contains("\"execution_time_ms\":1500"));
    }

    #[test]
    fn test_ingestion_operation_response_with_error() {
        let response = IngestionOperationResponse {
            id: "op-002".to_string(),
            source_name: None,
            status: "failed".to_string(),
            error: Some("Parse error".to_string()),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            execution_time_ms: 100,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"status\":\"failed\""));
        assert!(json.contains("\"error\":\"Parse error\""));
    }

    #[test]
    fn test_list_ingestion_operations_response_serialization() {
        let response = ListIngestionOperationsResponse {
            operations: vec![],
            total: 25,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"operations\":[]"));
        assert!(json.contains("\"total\":25"));
    }
}
