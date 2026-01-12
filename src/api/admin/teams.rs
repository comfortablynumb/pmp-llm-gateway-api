//! Team management admin endpoints

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::api::middleware::RequireAdmin;
use crate::api::state::AppState;
use crate::api::types::ApiError;
use crate::domain::team::{Team, TeamStatus};
use crate::infrastructure::team::{CreateTeamRequest, UpdateTeamRequest};

/// Request to create a new team
#[derive(Debug, Clone, Deserialize)]
pub struct CreateTeamApiRequest {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// Request to update a team
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateTeamApiRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

/// Team response for admin API
#[derive(Debug, Clone, Serialize)]
pub struct TeamResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

fn status_to_string(status: TeamStatus) -> String {
    match status {
        TeamStatus::Active => "active".to_string(),
        TeamStatus::Suspended => "suspended".to_string(),
    }
}

impl From<&Team> for TeamResponse {
    fn from(team: &Team) -> Self {
        Self {
            id: team.id().as_str().to_string(),
            name: team.name().to_string(),
            description: team.description().map(String::from),
            status: status_to_string(team.status()),
            created_at: team.created_at().to_rfc3339(),
            updated_at: team.updated_at().to_rfc3339(),
        }
    }
}

/// List teams response
#[derive(Debug, Clone, Serialize)]
pub struct ListTeamsResponse {
    pub teams: Vec<TeamResponse>,
    pub total: usize,
}

/// GET /admin/teams
pub async fn list_teams(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
) -> Result<Json<ListTeamsResponse>, ApiError> {
    debug!("Admin listing all teams");

    let teams = state.team_service.list(None).await.map_err(ApiError::from)?;

    let team_responses: Vec<TeamResponse> = teams.iter().map(TeamResponse::from).collect();
    let total = team_responses.len();

    Ok(Json(ListTeamsResponse {
        teams: team_responses,
        total,
    }))
}

/// POST /admin/teams
pub async fn create_team(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Json(request): Json<CreateTeamApiRequest>,
) -> Result<Json<TeamResponse>, ApiError> {
    debug!(id = %request.id, name = %request.name, "Admin creating team");

    let service_request = CreateTeamRequest {
        id: request.id,
        name: request.name,
        description: request.description,
    };

    let team = state
        .team_service
        .create(service_request)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(TeamResponse::from(&team)))
}

/// GET /admin/teams/:team_id
pub async fn get_team(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(team_id): Path<String>,
) -> Result<Json<TeamResponse>, ApiError> {
    debug!(team_id = %team_id, "Admin getting team");

    let team = state
        .team_service
        .get(&team_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::not_found(format!("Team '{}' not found", team_id)))?;

    Ok(Json(TeamResponse::from(&team)))
}

/// PUT /admin/teams/:team_id
pub async fn update_team(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(team_id): Path<String>,
    Json(request): Json<UpdateTeamApiRequest>,
) -> Result<Json<TeamResponse>, ApiError> {
    debug!(team_id = %team_id, "Admin updating team");

    let service_request = UpdateTeamRequest {
        name: request.name,
        description: request.description,
    };

    let team = state
        .team_service
        .update(&team_id, service_request)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(TeamResponse::from(&team)))
}

/// DELETE /admin/teams/:team_id
pub async fn delete_team(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(team_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    debug!(team_id = %team_id, "Admin deleting team");

    state
        .team_service
        .delete(&team_id)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(serde_json::json!({
        "deleted": true,
        "id": team_id
    })))
}

/// POST /admin/teams/:team_id/suspend
pub async fn suspend_team(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(team_id): Path<String>,
) -> Result<Json<TeamResponse>, ApiError> {
    debug!(team_id = %team_id, "Admin suspending team");

    let team = state
        .team_service
        .suspend(&team_id)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(TeamResponse::from(&team)))
}

/// POST /admin/teams/:team_id/activate
pub async fn activate_team(
    State(state): State<AppState>,
    RequireAdmin(_): RequireAdmin,
    Path(team_id): Path<String>,
) -> Result<Json<TeamResponse>, ApiError> {
    debug!(team_id = %team_id, "Admin activating team");

    let team = state
        .team_service
        .activate(&team_id)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(TeamResponse::from(&team)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::team::TeamId;

    #[test]
    fn test_create_team_request_deserialization() {
        let json = r#"{
            "id": "test-team",
            "name": "Test Team"
        }"#;

        let request: CreateTeamApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.id, "test-team");
        assert_eq!(request.name, "Test Team");
        assert!(request.description.is_none());
    }

    #[test]
    fn test_create_team_request_with_description() {
        let json = r#"{
            "id": "my-team",
            "name": "My Team",
            "description": "A test team"
        }"#;

        let request: CreateTeamApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.id, "my-team");
        assert_eq!(request.name, "My Team");
        assert_eq!(request.description, Some("A test team".to_string()));
    }

    #[test]
    fn test_update_team_request_partial() {
        let json = r#"{
            "name": "Updated Name"
        }"#;

        let request: UpdateTeamApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.name, Some("Updated Name".to_string()));
        assert!(request.description.is_none());
    }

    #[test]
    fn test_update_team_request_full() {
        let json = r#"{
            "name": "New Name",
            "description": "New Description"
        }"#;

        let request: UpdateTeamApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.name, Some("New Name".to_string()));
        assert_eq!(request.description, Some("New Description".to_string()));
    }

    #[test]
    fn test_update_team_request_empty() {
        let json = r#"{}"#;

        let request: UpdateTeamApiRequest = serde_json::from_str(json).unwrap();
        assert!(request.name.is_none());
        assert!(request.description.is_none());
    }

    #[test]
    fn test_status_to_string_active() {
        assert_eq!(status_to_string(TeamStatus::Active), "active");
    }

    #[test]
    fn test_status_to_string_suspended() {
        assert_eq!(status_to_string(TeamStatus::Suspended), "suspended");
    }

    #[test]
    fn test_team_response_from() {
        let id = TeamId::new("test-team").unwrap();
        let team = Team::new(id, "Test Team").unwrap();

        let response = TeamResponse::from(&team);

        assert_eq!(response.id, "test-team");
        assert_eq!(response.name, "Test Team");
        assert!(response.description.is_none());
        assert_eq!(response.status, "active");
    }

    #[test]
    fn test_team_response_from_with_description() {
        let id = TeamId::new("my-team").unwrap();
        let team = Team::new(id, "My Team")
            .unwrap()
            .with_description("A team description");

        let response = TeamResponse::from(&team);

        assert_eq!(response.id, "my-team");
        assert_eq!(response.name, "My Team");
        assert_eq!(response.description, Some("A team description".to_string()));
        assert_eq!(response.status, "active");
    }

    #[test]
    fn test_team_response_serialization() {
        let id = TeamId::new("test-team").unwrap();
        let team = Team::new(id, "Test Team").unwrap();
        let response = TeamResponse::from(&team);

        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("\"id\":\"test-team\""));
        assert!(json.contains("\"name\":\"Test Team\""));
        assert!(json.contains("\"status\":\"active\""));
        assert!(json.contains("\"created_at\":"));
        assert!(json.contains("\"updated_at\":"));
    }

    #[test]
    fn test_list_teams_response_serialization() {
        let id = TeamId::new("team-1").unwrap();
        let team = Team::new(id, "Team One").unwrap();
        let response = TeamResponse::from(&team);

        let list_response = ListTeamsResponse {
            teams: vec![response],
            total: 1,
        };

        let json = serde_json::to_string(&list_response).unwrap();

        assert!(json.contains("\"teams\":"));
        assert!(json.contains("\"total\":1"));
        assert!(json.contains("\"id\":\"team-1\""));
    }

    #[test]
    fn test_list_teams_response_empty() {
        let list_response = ListTeamsResponse {
            teams: vec![],
            total: 0,
        };

        let json = serde_json::to_string(&list_response).unwrap();

        assert!(json.contains("\"teams\":[]"));
        assert!(json.contains("\"total\":0"));
    }
}
