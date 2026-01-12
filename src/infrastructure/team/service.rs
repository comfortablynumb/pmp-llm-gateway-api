//! Team service for team management

use std::sync::Arc;

use tracing::{debug, info};

use crate::domain::team::{Team, TeamId, TeamQuery, TeamRepository, TeamStatus, validate_team_name};
use crate::domain::DomainError;

/// Request for creating a new team
#[derive(Debug, Clone)]
pub struct CreateTeamRequest {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

/// Request for updating a team
#[derive(Debug, Clone)]
pub struct UpdateTeamRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

/// Team service for managing teams
#[derive(Debug)]
pub struct TeamService<R: TeamRepository> {
    repository: Arc<R>,
}

impl<R: TeamRepository> TeamService<R> {
    /// Create a new team service
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }

    /// Create a new team
    pub async fn create(&self, request: CreateTeamRequest) -> Result<Team, DomainError> {
        info!(id = %request.id, name = %request.name, "Creating team");

        // Validate name
        validate_team_name(&request.name)
            .map_err(|e| DomainError::validation(e.to_string()))?;

        // Parse team ID
        let team_id = TeamId::new(&request.id)
            .map_err(|e| DomainError::invalid_id(e.to_string()))?;

        // Check if team already exists
        if self.repository.exists(&team_id).await? {
            return Err(DomainError::conflict(format!(
                "Team '{}' already exists",
                request.id
            )));
        }

        // Create the team
        let mut team = Team::new(team_id, &request.name)
            .map_err(|e| DomainError::validation(e.to_string()))?;

        if let Some(desc) = request.description {
            team.set_description(Some(desc));
        }

        self.repository.create(team).await
    }

    /// Get a team by ID
    pub async fn get(&self, id: &str) -> Result<Option<Team>, DomainError> {
        let team_id = TeamId::new(id).map_err(|e| DomainError::invalid_id(e.to_string()))?;
        self.repository.get(&team_id).await
    }

    /// Get a team by ID (TeamId)
    pub async fn get_by_id(&self, id: &TeamId) -> Result<Option<Team>, DomainError> {
        self.repository.get(id).await
    }

    /// List all teams
    pub async fn list(&self, query: Option<TeamQuery>) -> Result<Vec<Team>, DomainError> {
        self.repository.list(&query.unwrap_or_default()).await
    }

    /// Count teams
    pub async fn count(&self, query: Option<TeamQuery>) -> Result<usize, DomainError> {
        self.repository.count(&query.unwrap_or_default()).await
    }

    /// Update a team
    pub async fn update(&self, id: &str, request: UpdateTeamRequest) -> Result<Team, DomainError> {
        info!(id = %id, "Updating team");

        let team_id = TeamId::new(id).map_err(|e| DomainError::invalid_id(e.to_string()))?;

        let mut team = self
            .repository
            .get(&team_id)
            .await?
            .ok_or_else(|| DomainError::not_found(format!("Team '{}' not found", id)))?;

        if let Some(name) = request.name {
            team.set_name(&name)
                .map_err(|e| DomainError::validation(e.to_string()))?;
        }

        if let Some(desc) = request.description {
            team.set_description(Some(desc));
        }

        self.repository.update(team).await
    }

    /// Suspend a team
    pub async fn suspend(&self, id: &str) -> Result<Team, DomainError> {
        info!(id = %id, "Suspending team");

        let team_id = TeamId::new(id).map_err(|e| DomainError::invalid_id(e.to_string()))?;

        let mut team = self
            .repository
            .get(&team_id)
            .await?
            .ok_or_else(|| DomainError::not_found(format!("Team '{}' not found", id)))?;

        // Prevent suspending the administrators team
        if team_id.as_str() == TeamId::ADMINISTRATORS {
            return Err(DomainError::validation(
                "Cannot suspend the administrators team",
            ));
        }

        team.suspend();
        self.repository.update(team).await
    }

    /// Activate a suspended team
    pub async fn activate(&self, id: &str) -> Result<Team, DomainError> {
        info!(id = %id, "Activating team");

        let team_id = TeamId::new(id).map_err(|e| DomainError::invalid_id(e.to_string()))?;

        let mut team = self
            .repository
            .get(&team_id)
            .await?
            .ok_or_else(|| DomainError::not_found(format!("Team '{}' not found", id)))?;

        if team.status() != TeamStatus::Suspended {
            return Err(DomainError::validation("Only suspended teams can be activated"));
        }

        team.activate();
        self.repository.update(team).await
    }

    /// Delete a team
    pub async fn delete(&self, id: &str) -> Result<bool, DomainError> {
        info!(id = %id, "Deleting team");

        let team_id = TeamId::new(id).map_err(|e| DomainError::invalid_id(e.to_string()))?;

        // Prevent deleting the administrators team
        if team_id.as_str() == TeamId::ADMINISTRATORS {
            return Err(DomainError::validation(
                "Cannot delete the administrators team",
            ));
        }

        self.repository.delete(&team_id).await
    }

    /// Ensure the administrators team exists
    pub async fn ensure_administrators_team(&self) -> Result<Team, DomainError> {
        let admin_id = TeamId::administrators();

        if let Some(team) = self.repository.get(&admin_id).await? {
            debug!("Administrators team already exists");
            return Ok(team);
        }

        info!("Creating administrators team");

        let team = Team::new(admin_id, "Administrators")
            .map_err(|e| DomainError::validation(e.to_string()))?
            .with_description("System administrators team");

        self.repository.create(team).await
    }

    /// Check if a team exists
    pub async fn exists(&self, id: &str) -> Result<bool, DomainError> {
        let team_id = TeamId::new(id).map_err(|e| DomainError::invalid_id(e.to_string()))?;
        self.repository.exists(&team_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::team::Team;
    use crate::infrastructure::storage::InMemoryStorage;
    use crate::infrastructure::team::StorageTeamRepository;

    fn create_service() -> TeamService<StorageTeamRepository> {
        let storage = Arc::new(InMemoryStorage::<Team>::new());
        let repository = Arc::new(StorageTeamRepository::new(storage));
        TeamService::new(repository)
    }

    #[tokio::test]
    async fn test_create_team() {
        let service = create_service();

        let request = CreateTeamRequest {
            id: "test-team".to_string(),
            name: "Test Team".to_string(),
            description: Some("A test team".to_string()),
        };

        let team = service.create(request).await.unwrap();
        assert_eq!(team.name(), "Test Team");
        assert_eq!(team.description(), Some("A test team"));
        assert_eq!(team.status(), TeamStatus::Active);
    }

    #[tokio::test]
    async fn test_create_team_invalid_name() {
        let service = create_service();

        let request = CreateTeamRequest {
            id: "test-team".to_string(),
            name: "".to_string(), // Invalid - empty
            description: None,
        };

        let result = service.create(request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_duplicate_team() {
        let service = create_service();

        let request1 = CreateTeamRequest {
            id: "test-team".to_string(),
            name: "Test Team 1".to_string(),
            description: None,
        };

        let request2 = CreateTeamRequest {
            id: "test-team".to_string(),
            name: "Test Team 2".to_string(),
            description: None,
        };

        service.create(request1).await.unwrap();

        let result = service.create(request2).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_team() {
        let service = create_service();

        let request = CreateTeamRequest {
            id: "test-team".to_string(),
            name: "Test Team".to_string(),
            description: None,
        };

        service.create(request).await.unwrap();

        let team = service.get("test-team").await.unwrap();
        assert!(team.is_some());
        assert_eq!(team.unwrap().name(), "Test Team");
    }

    #[tokio::test]
    async fn test_update_team() {
        let service = create_service();

        let request = CreateTeamRequest {
            id: "test-team".to_string(),
            name: "Test Team".to_string(),
            description: None,
        };

        service.create(request).await.unwrap();

        let update = UpdateTeamRequest {
            name: Some("Updated Team".to_string()),
            description: Some("New description".to_string()),
        };

        let updated = service.update("test-team", update).await.unwrap();
        assert_eq!(updated.name(), "Updated Team");
        assert_eq!(updated.description(), Some("New description"));
    }

    #[tokio::test]
    async fn test_suspend_and_activate() {
        let service = create_service();

        let request = CreateTeamRequest {
            id: "test-team".to_string(),
            name: "Test Team".to_string(),
            description: None,
        };

        service.create(request).await.unwrap();

        // Suspend
        let suspended = service.suspend("test-team").await.unwrap();
        assert_eq!(suspended.status(), TeamStatus::Suspended);

        // Activate
        let activated = service.activate("test-team").await.unwrap();
        assert_eq!(activated.status(), TeamStatus::Active);
    }

    #[tokio::test]
    async fn test_cannot_suspend_administrators() {
        let service = create_service();
        service.ensure_administrators_team().await.unwrap();

        let result = service.suspend(TeamId::ADMINISTRATORS).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cannot_delete_administrators() {
        let service = create_service();
        service.ensure_administrators_team().await.unwrap();

        let result = service.delete(TeamId::ADMINISTRATORS).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_team() {
        let service = create_service();

        let request = CreateTeamRequest {
            id: "test-team".to_string(),
            name: "Test Team".to_string(),
            description: None,
        };

        service.create(request).await.unwrap();

        let deleted = service.delete("test-team").await.unwrap();
        assert!(deleted);

        let team = service.get("test-team").await.unwrap();
        assert!(team.is_none());
    }

    #[tokio::test]
    async fn test_ensure_administrators_team() {
        let service = create_service();

        // First call creates the team
        let team1 = service.ensure_administrators_team().await.unwrap();
        assert_eq!(team1.id().as_str(), TeamId::ADMINISTRATORS);
        assert_eq!(team1.name(), "Administrators");

        // Second call returns existing team
        let team2 = service.ensure_administrators_team().await.unwrap();
        assert_eq!(team2.id().as_str(), team1.id().as_str());
    }

    #[tokio::test]
    async fn test_list_teams() {
        let service = create_service();

        service
            .create(CreateTeamRequest {
                id: "team-a".to_string(),
                name: "Team A".to_string(),
                description: None,
            })
            .await
            .unwrap();

        service
            .create(CreateTeamRequest {
                id: "team-b".to_string(),
                name: "Team B".to_string(),
                description: None,
            })
            .await
            .unwrap();

        let teams = service.list(None).await.unwrap();
        assert_eq!(teams.len(), 2);
    }

    #[tokio::test]
    async fn test_count_teams() {
        let service = create_service();

        service
            .create(CreateTeamRequest {
                id: "team-a".to_string(),
                name: "Team A".to_string(),
                description: None,
            })
            .await
            .unwrap();

        service
            .create(CreateTeamRequest {
                id: "team-b".to_string(),
                name: "Team B".to_string(),
                description: None,
            })
            .await
            .unwrap();

        let count = service.count(None).await.unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_exists() {
        let service = create_service();

        assert!(!service.exists("test-team").await.unwrap());

        service
            .create(CreateTeamRequest {
                id: "test-team".to_string(),
                name: "Test Team".to_string(),
                description: None,
            })
            .await
            .unwrap();

        assert!(service.exists("test-team").await.unwrap());
    }
}
