//! Validation helpers for ingestion

use crate::domain::DomainError;

use super::pipeline::ParserType;

/// Detect parser type from filename extension
pub fn detect_parser_from_filename(filename: &str) -> Option<ParserType> {
    let ext = filename.rsplit('.').next()?.to_lowercase();

    match ext.as_str() {
        "txt" | "text" => Some(ParserType::PlainText),
        "md" | "markdown" => Some(ParserType::Markdown),
        "html" | "htm" => Some(ParserType::Html),
        "json" => Some(ParserType::Json),
        "pdf" => Some(ParserType::Pdf),
        _ => None,
    }
}

/// Detect parser type from MIME type
pub fn detect_parser_from_mime(mime: &str) -> Option<ParserType> {
    let mime_lower = mime.to_lowercase();

    if mime_lower.starts_with("text/plain") {
        return Some(ParserType::PlainText);
    }

    if mime_lower.starts_with("text/markdown") || mime_lower.starts_with("text/x-markdown") {
        return Some(ParserType::Markdown);
    }

    if mime_lower.starts_with("text/html") {
        return Some(ParserType::Html);
    }

    if mime_lower.starts_with("application/json") {
        return Some(ParserType::Json);
    }

    if mime_lower.starts_with("application/pdf") {
        return Some(ParserType::Pdf);
    }

    None
}

/// Validate document ID format
pub fn validate_document_id(id: &str) -> Result<(), DomainError> {
    if id.is_empty() {
        return Err(DomainError::validation("Document ID cannot be empty"));
    }

    if id.len() > 255 {
        return Err(DomainError::validation(
            "Document ID cannot exceed 255 characters",
        ));
    }

    Ok(())
}

/// Validate chunk size parameters
pub fn validate_chunk_params(chunk_size: usize, chunk_overlap: usize) -> Result<(), DomainError> {
    if chunk_size == 0 {
        return Err(DomainError::validation("Chunk size must be greater than 0"));
    }

    if chunk_size > 100_000 {
        return Err(DomainError::validation(
            "Chunk size cannot exceed 100,000 characters",
        ));
    }

    if chunk_overlap >= chunk_size {
        return Err(DomainError::validation(
            "Chunk overlap must be less than chunk size",
        ));
    }

    Ok(())
}

/// Validate batch size
pub fn validate_batch_size(batch_size: usize) -> Result<(), DomainError> {
    if batch_size == 0 {
        return Err(DomainError::validation("Batch size must be greater than 0"));
    }

    if batch_size > 1000 {
        return Err(DomainError::validation("Batch size cannot exceed 1000"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_parser_from_filename() {
        assert_eq!(
            detect_parser_from_filename("file.txt"),
            Some(ParserType::PlainText)
        );
        assert_eq!(
            detect_parser_from_filename("file.TXT"),
            Some(ParserType::PlainText)
        );
        assert_eq!(
            detect_parser_from_filename("readme.md"),
            Some(ParserType::Markdown)
        );
        assert_eq!(
            detect_parser_from_filename("page.html"),
            Some(ParserType::Html)
        );
        assert_eq!(
            detect_parser_from_filename("data.json"),
            Some(ParserType::Json)
        );
        assert_eq!(
            detect_parser_from_filename("doc.pdf"),
            Some(ParserType::Pdf)
        );
        assert_eq!(detect_parser_from_filename("unknown.xyz"), None);
        assert_eq!(detect_parser_from_filename("noextension"), None);
    }

    #[test]
    fn test_detect_parser_from_mime() {
        assert_eq!(
            detect_parser_from_mime("text/plain"),
            Some(ParserType::PlainText)
        );
        assert_eq!(
            detect_parser_from_mime("text/plain; charset=utf-8"),
            Some(ParserType::PlainText)
        );
        assert_eq!(
            detect_parser_from_mime("text/markdown"),
            Some(ParserType::Markdown)
        );
        assert_eq!(
            detect_parser_from_mime("text/html"),
            Some(ParserType::Html)
        );
        assert_eq!(
            detect_parser_from_mime("application/json"),
            Some(ParserType::Json)
        );
        assert_eq!(
            detect_parser_from_mime("application/pdf"),
            Some(ParserType::Pdf)
        );
        assert_eq!(detect_parser_from_mime("image/png"), None);
    }

    #[test]
    fn test_validate_document_id() {
        assert!(validate_document_id("valid-id").is_ok());
        assert!(validate_document_id("").is_err());
        assert!(validate_document_id(&"a".repeat(256)).is_err());
    }

    #[test]
    fn test_validate_chunk_params() {
        assert!(validate_chunk_params(1000, 200).is_ok());
        assert!(validate_chunk_params(0, 0).is_err());
        assert!(validate_chunk_params(100, 100).is_err());
        assert!(validate_chunk_params(100, 150).is_err());
        assert!(validate_chunk_params(100_001, 0).is_err());
    }

    #[test]
    fn test_validate_batch_size() {
        assert!(validate_batch_size(100).is_ok());
        assert!(validate_batch_size(0).is_err());
        assert!(validate_batch_size(1001).is_err());
    }
}
