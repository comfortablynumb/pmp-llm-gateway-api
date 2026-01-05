use async_trait::async_trait;
use bytes::Bytes;
use futures::Stream;
use std::pin::Pin;

use crate::domain::DomainError;

/// Stream type for HTTP responses
pub type ByteStream = Pin<Box<dyn Stream<Item = Result<Bytes, DomainError>> + Send>>;

/// Trait for HTTP client operations (for mocking)
#[async_trait]
pub trait HttpClientTrait: Send + Sync + std::fmt::Debug {
    async fn post_json(
        &self,
        url: &str,
        headers: Vec<(&str, &str)>,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value, DomainError>;

    async fn post_json_stream(
        &self,
        url: &str,
        headers: Vec<(&str, &str)>,
        body: &serde_json::Value,
    ) -> Result<ByteStream, DomainError>;
}

/// Real HTTP client using reqwest
#[derive(Debug, Clone)]
pub struct HttpClient {
    client: reqwest::Client,
}

impl HttpClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    pub fn with_timeout(timeout: std::time::Duration) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(timeout)
                .build()
                .expect("Failed to build HTTP client"),
        }
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl HttpClientTrait for HttpClient {
    async fn post_json(
        &self,
        url: &str,
        headers: Vec<(&str, &str)>,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value, DomainError> {
        let mut request = self.client.post(url);

        for (key, value) in headers {
            request = request.header(key, value);
        }

        let response = request
            .json(body)
            .send()
            .await
            .map_err(|e| DomainError::provider("http", format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            return Err(DomainError::provider(
                "http",
                format!("HTTP {}: {}", status, error_body),
            ));
        }

        response
            .json()
            .await
            .map_err(|e| DomainError::provider("http", format!("Failed to parse response: {}", e)))
    }

    async fn post_json_stream(
        &self,
        url: &str,
        headers: Vec<(&str, &str)>,
        body: &serde_json::Value,
    ) -> Result<ByteStream, DomainError> {
        let mut request = self.client.post(url);

        for (key, value) in headers {
            request = request.header(key, value);
        }

        let response = request
            .json(body)
            .send()
            .await
            .map_err(|e| DomainError::provider("http", format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            return Err(DomainError::provider(
                "http",
                format!("HTTP {}: {}", status, error_body),
            ));
        }

        use futures::StreamExt;
        let stream = response.bytes_stream().map(|result| {
            result.map_err(|e| DomainError::provider("http", format!("Stream error: {}", e)))
        });

        Ok(Box::pin(stream))
    }
}

#[cfg(test)]
pub mod mock {
    use super::*;
    use futures::stream;
    use std::collections::HashMap;
    use std::sync::RwLock;

    #[derive(Debug)]
    pub struct MockHttpClient {
        responses: RwLock<HashMap<String, serde_json::Value>>,
        stream_responses: RwLock<HashMap<String, Vec<Bytes>>>,
        errors: RwLock<HashMap<String, String>>,
    }

    impl MockHttpClient {
        pub fn new() -> Self {
            Self {
                responses: RwLock::new(HashMap::new()),
                stream_responses: RwLock::new(HashMap::new()),
                errors: RwLock::new(HashMap::new()),
            }
        }

        pub fn with_response(self, url: impl Into<String>, response: serde_json::Value) -> Self {
            self.responses.write().unwrap().insert(url.into(), response);
            self
        }

        pub fn with_stream_response(self, url: impl Into<String>, chunks: Vec<Bytes>) -> Self {
            self.stream_responses
                .write()
                .unwrap()
                .insert(url.into(), chunks);
            self
        }

        pub fn with_error(self, url: impl Into<String>, error: impl Into<String>) -> Self {
            self.errors.write().unwrap().insert(url.into(), error.into());
            self
        }
    }

    impl Default for MockHttpClient {
        fn default() -> Self {
            Self::new()
        }
    }

    #[async_trait]
    impl HttpClientTrait for MockHttpClient {
        async fn post_json(
            &self,
            url: &str,
            _headers: Vec<(&str, &str)>,
            _body: &serde_json::Value,
        ) -> Result<serde_json::Value, DomainError> {
            if let Some(error) = self.errors.read().unwrap().get(url) {
                return Err(DomainError::provider("mock", error));
            }

            self.responses
                .read()
                .unwrap()
                .get(url)
                .cloned()
                .ok_or_else(|| DomainError::provider("mock", format!("No mock response for {}", url)))
        }

        async fn post_json_stream(
            &self,
            url: &str,
            _headers: Vec<(&str, &str)>,
            _body: &serde_json::Value,
        ) -> Result<ByteStream, DomainError> {
            if let Some(error) = self.errors.read().unwrap().get(url) {
                return Err(DomainError::provider("mock", error));
            }

            let chunks = self
                .stream_responses
                .read()
                .unwrap()
                .get(url)
                .cloned()
                .unwrap_or_default();

            let stream = stream::iter(chunks.into_iter().map(Ok));
            Ok(Box::pin(stream))
        }
    }
}
