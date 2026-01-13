//! Custom JSON extractor that returns errors as JSON

use axum::{
    extract::{FromRequest, Request},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json as AxumJson,
};
use serde::de::DeserializeOwned;

use super::error::{ApiErrorDetail, ApiErrorResponse, ApiErrorType};

/// Custom JSON extractor that converts all rejection errors to JSON format
///
/// This wrapper around `axum::Json` ensures that deserialization errors
/// are returned as JSON responses matching our API error format.
#[derive(Debug, Clone, Copy, Default)]
pub struct Json<T>(pub T);

impl<T> Json<T> {
    /// Consume the extractor and return the inner value
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> std::ops::Deref for Json<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for Json<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// JSON rejection error that returns API error format
#[derive(Debug)]
pub struct JsonRejection {
    status: StatusCode,
    message: String,
}

impl IntoResponse for JsonRejection {
    fn into_response(self) -> Response {
        let response = ApiErrorResponse {
            error: ApiErrorDetail {
                message: self.message,
                error_type: ApiErrorType::InvalidRequestError,
                param: None,
                code: Some("json_parse_error".to_string()),
            },
        };

        (self.status, AxumJson(response)).into_response()
    }
}

impl<S, T> FromRequest<S> for Json<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = JsonRejection;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match AxumJson::<T>::from_request(req, state).await {
            Ok(AxumJson(value)) => Ok(Json(value)),
            Err(rejection) => {
                let message = format_rejection_message(&rejection);
                let status = rejection.status();

                Err(JsonRejection { status, message })
            }
        }
    }
}

/// Format the rejection message to be more user-friendly
fn format_rejection_message(rejection: &axum::extract::rejection::JsonRejection) -> String {
    use axum::extract::rejection::JsonRejection::*;

    match rejection {
        JsonDataError(err) => {
            // Extract the serde error message which contains field info
            let msg = err.body_text();
            format!("Invalid JSON data: {}", msg)
        }
        JsonSyntaxError(err) => {
            format!("Invalid JSON syntax: {}", err.body_text())
        }
        MissingJsonContentType(_) => {
            "Missing Content-Type header. Expected 'application/json'.".to_string()
        }
        BytesRejection(err) => {
            format!("Failed to read request body: {}", err.body_text())
        }
        _ => "Invalid JSON request".to_string(),
    }
}

impl<T> IntoResponse for Json<T>
where
    T: serde::Serialize,
{
    fn into_response(self) -> Response {
        AxumJson(self.0).into_response()
    }
}

impl<T> From<T> for Json<T> {
    fn from(value: T) -> Self {
        Json(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct TestStruct {
        name: String,
    }

    #[test]
    fn test_json_rejection_into_response() {
        let rejection = JsonRejection {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            message: "Test error".to_string(),
        };

        let response = rejection.into_response();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[test]
    fn test_json_deref() {
        let json = Json("hello".to_string());
        assert_eq!(*json, "hello");
    }

    #[test]
    fn test_json_into_inner() {
        let json = Json(42);
        assert_eq!(json.into_inner(), 42);
    }
}
