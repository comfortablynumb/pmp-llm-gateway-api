//! Team repository trait

use async_trait::async_trait;

use super::entity::{Team, TeamId};
use crate::domain::DomainError;

/// Query parameters for listing teams
#[derive(Debug, Clone, Default)]
pub struct TeamQuery {
    /// Filter by status
    pub status: Option<String>,
    /// Maximum number of results
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
}

impl TeamQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_status(mut self, status: impl Into<String>) -> Self {
        self.status = Some(status.into());
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }
}

/// Repository for managing teams
#[async_trait]
pub trait TeamRepository: Send + Sync + std::fmt::Debug {
    /// Get a team by ID
    async fn get(&self, id: &TeamId) -> Result<Option<Team>, DomainError>;

    /// Create a new team
    async fn create(&self, team: Team) -> Result<Team, DomainError>;

    /// Update an existing team
    async fn update(&self, team: Team) -> Result<Team, DomainError>;

    /// Delete a team by ID
    async fn delete(&self, id: &TeamId) -> Result<bool, DomainError>;

    /// List all teams
    async fn list(&self, query: &TeamQuery) -> Result<Vec<Team>, DomainError>;

    /// Count teams matching query
    async fn count(&self, query: &TeamQuery) -> Result<usize, DomainError>;

    /// Check if a team exists
    async fn exists(&self, id: &TeamId) -> Result<bool, DomainError>;
}

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::collections::HashMap;
    use std::sync::RwLock;

    /// Mock implementation for testing
    #[derive(Debug, Default)]
    pub struct MockTeamRepository {
        teams: RwLock<HashMap<String, Team>>,
    }

    impl MockTeamRepository {
        pub fn new() -> Self {
            Self::default()
        }
    }

    #[async_trait]
    impl TeamRepository for MockTeamRepository {
        async fn get(&self, id: &TeamId) -> Result<Option<Team>, DomainError> {
            let teams = self.teams.read().unwrap();
            Ok(teams.get(id.as_str()).cloned())
        }

        async fn create(&self, team: Team) -> Result<Team, DomainError> {
            let mut teams = self.teams.write().unwrap();

            if teams.contains_key(team.id().as_str()) {
                return Err(DomainError::conflict(format!(
                    "Team '{}' already exists",
                    team.id()
                )));
            }

            teams.insert(team.id().as_str().to_string(), team.clone());
            Ok(team)
        }

        async fn update(&self, team: Team) -> Result<Team, DomainError> {
            let mut teams = self.teams.write().unwrap();

            if !teams.contains_key(team.id().as_str()) {
                return Err(DomainError::not_found(format!(
                    "Team '{}' not found",
                    team.id()
                )));
            }

            teams.insert(team.id().as_str().to_string(), team.clone());
            Ok(team)
        }

        async fn delete(&self, id: &TeamId) -> Result<bool, DomainError> {
            let mut teams = self.teams.write().unwrap();
            Ok(teams.remove(id.as_str()).is_some())
        }

        async fn list(&self, query: &TeamQuery) -> Result<Vec<Team>, DomainError> {
            let teams = self.teams.read().unwrap();
            let mut result: Vec<Team> = teams.values().cloned().collect();

            // Filter by status
            if let Some(ref status) = query.status {
                result.retain(|t| t.status().to_string() == *status);
            }

            // Sort by name
            result.sort_by(|a, b| a.name().cmp(b.name()));

            // Apply pagination
            let offset = query.offset.unwrap_or(0);

            if offset < result.len() {
                result = result.into_iter().skip(offset).collect();
            } else {
                result.clear();
            }

            if let Some(limit) = query.limit {
                result.truncate(limit);
            }

            Ok(result)
        }

        async fn count(&self, query: &TeamQuery) -> Result<usize, DomainError> {
            let teams = self.teams.read().unwrap();
            let mut count = teams.len();

            if let Some(ref status) = query.status {
                count = teams
                    .values()
                    .filter(|t| t.status().to_string() == *status)
                    .count();
            }

            Ok(count)
        }

        async fn exists(&self, id: &TeamId) -> Result<bool, DomainError> {
            let teams = self.teams.read().unwrap();
            Ok(teams.contains_key(id.as_str()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::mock::MockTeamRepository;
    use super::*;

    #[tokio::test]
    async fn test_mock_create_and_get() {
        let repo = MockTeamRepository::new();
        let id = TeamId::new("test-team").unwrap();
        let team = Team::new(id.clone(), "Test Team").unwrap();

        let created = repo.create(team).await.unwrap();
        assert_eq!(created.id().as_str(), "test-team");

        let fetched = repo.get(&id).await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().name(), "Test Team");
    }

    #[tokio::test]
    async fn test_mock_create_duplicate() {
        let repo = MockTeamRepository::new();
        let id = TeamId::new("test-team").unwrap();
        let team1 = Team::new(id.clone(), "Test Team 1").unwrap();
        let team2 = Team::new(id, "Test Team 2").unwrap();

        repo.create(team1).await.unwrap();
        let result = repo.create(team2).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_update() {
        let repo = MockTeamRepository::new();
        let id = TeamId::new("test-team").unwrap();
        let team = Team::new(id.clone(), "Test Team").unwrap();

        repo.create(team).await.unwrap();

        let mut updated_team = repo.get(&id).await.unwrap().unwrap();
        updated_team.set_name("Updated Team").unwrap();
        repo.update(updated_team).await.unwrap();

        let fetched = repo.get(&id).await.unwrap().unwrap();
        assert_eq!(fetched.name(), "Updated Team");
    }

    #[tokio::test]
    async fn test_mock_delete() {
        let repo = MockTeamRepository::new();
        let id = TeamId::new("test-team").unwrap();
        let team = Team::new(id.clone(), "Test Team").unwrap();

        repo.create(team).await.unwrap();
        assert!(repo.exists(&id).await.unwrap());

        let deleted = repo.delete(&id).await.unwrap();
        assert!(deleted);
        assert!(!repo.exists(&id).await.unwrap());
    }

    #[tokio::test]
    async fn test_mock_list() {
        let repo = MockTeamRepository::new();

        let team1 = Team::new(TeamId::new("team-a").unwrap(), "Team A").unwrap();
        let team2 = Team::new(TeamId::new("team-b").unwrap(), "Team B").unwrap();

        repo.create(team1).await.unwrap();
        repo.create(team2).await.unwrap();

        let teams = repo.list(&TeamQuery::new()).await.unwrap();
        assert_eq!(teams.len(), 2);
    }

    #[tokio::test]
    async fn test_mock_list_with_pagination() {
        let repo = MockTeamRepository::new();

        for i in 0..5 {
            let team =
                Team::new(TeamId::new(format!("team-{}", i)).unwrap(), format!("Team {}", i))
                    .unwrap();
            repo.create(team).await.unwrap();
        }

        let query = TeamQuery::new().with_limit(2).with_offset(1);
        let teams = repo.list(&query).await.unwrap();
        assert_eq!(teams.len(), 2);
    }

    #[tokio::test]
    async fn test_mock_count() {
        let repo = MockTeamRepository::new();

        let team1 = Team::new(TeamId::new("team-a").unwrap(), "Team A").unwrap();
        let team2 = Team::new(TeamId::new("team-b").unwrap(), "Team B").unwrap();

        repo.create(team1).await.unwrap();
        repo.create(team2).await.unwrap();

        let count = repo.count(&TeamQuery::new()).await.unwrap();
        assert_eq!(count, 2);
    }
}
