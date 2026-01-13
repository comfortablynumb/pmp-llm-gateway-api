//! Authentication API endpoints
//!
//! Provides login, logout, and user info endpoints for JWT-based authentication.

use axum::{
    extract::State,
    routing::{get, post},
    Router,
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::api::middleware::RequireUser;
use crate::api::state::AppState;
use crate::api::types::{ApiError, Json};

/// Create the authentication router
pub fn create_auth_router() -> Router<AppState> {
    Router::new()
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/me", get(get_current_user))
}

/// Login request
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Login response
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserResponse,
    pub expires_at: String,
}

/// User response (safe to expose)
#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: String,
    pub username: String,
    pub status: String,
    pub created_at: String,
    pub last_login_at: Option<String>,
}

impl UserResponse {
    fn from_user(user: &crate::domain::user::User) -> Self {
        Self {
            id: user.id().as_str().to_string(),
            username: user.username().to_string(),
            status: format!("{:?}", user.status()).to_lowercase(),
            created_at: user.created_at().to_rfc3339(),
            last_login_at: user.last_login_at().map(|t| t.to_rfc3339()),
        }
    }
}

/// Login with username and password
///
/// POST /auth/login
///
/// Returns a JWT token on successful authentication.
pub async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, ApiError> {
    // Authenticate user
    let user = state
        .user_service
        .authenticate(&request.username, &request.password)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .ok_or_else(|| ApiError::unauthorized("Invalid username or password"))?;

    // Generate JWT token
    let token = state
        .jwt_service
        .generate(&user)
        .map_err(|e| ApiError::internal(e.to_string()))?;

    // Calculate expiration time
    let expires_at = Utc::now() + Duration::hours(state.jwt_service.expiration_hours() as i64);

    Ok(Json(LoginResponse {
        token,
        user: UserResponse::from_user(&user),
        expires_at: expires_at.to_rfc3339(),
    }))
}

/// Logout (client-side only for stateless JWT)
///
/// POST /auth/logout
///
/// For JWT tokens, logout is handled client-side by discarding the token.
/// This endpoint exists for API consistency.
pub async fn logout(_user: RequireUser) -> Result<Json<LogoutResponse>, ApiError> {
    Ok(Json(LogoutResponse {
        message: "Logged out successfully".to_string(),
    }))
}

/// Logout response
#[derive(Debug, Serialize)]
pub struct LogoutResponse {
    pub message: String,
}

/// Get current authenticated user
///
/// GET /auth/me
///
/// Returns information about the currently authenticated user.
pub async fn get_current_user(
    RequireUser(user): RequireUser,
) -> Result<Json<UserResponse>, ApiError> {
    Ok(Json(UserResponse::from_user(&user)))
}
