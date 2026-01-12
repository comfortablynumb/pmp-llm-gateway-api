//! Storage-backed team repository implementation

use async_trait::async_trait;
use std::sync::Arc;

use crate::domain::storage::Storage;
use crate::domain::team::{Team, TeamId, TeamQuery, TeamRepository};
use crate::domain::DomainError;

/// Storage-backed implementation of TeamRepository
#[derive(Debug)]
pub struct StorageTeamRepository {
    storage: Arc<dyn Storage<Team>>,
}

impl StorageTeamRepository {
    /// Create a new storage-backed repository
    pub fn new(storage: Arc<dyn Storage<Team>>) -> Self {
        Self { storage }
    }
}

#[async_trait]
impl TeamRepository for StorageTeamRepository {
    async fn get(&self, id: &TeamId) -> Result<Option<Team>, DomainError> {
        self.storage.get(id).await
    }

    async fn create(&self, team: Team) -> Result<Team, DomainError> {
        if self.storage.exists(team.id()).await? {
            return Err(DomainError::conflict(format!(
                "Team '{}' already exists",
                team.id().as_str()
            )));
        }

        self.storage.create(team).await
    }

    async fn update(&self, team: Team) -> Result<Team, DomainError> {
        if !self.storage.exists(team.id()).await? {
            return Err(DomainError::not_found(format!(
                "Team '{}' not found",
                team.id().as_str()
            )));
        }

        self.storage.update(team).await
    }

    async fn delete(&self, id: &TeamId) -> Result<bool, DomainError> {
        self.storage.delete(id).await
    }

    async fn list(&self, query: &TeamQuery) -> Result<Vec<Team>, DomainError> {
        let all_teams = self.storage.list().await?;
        let mut result: Vec<Team> = filter_teams(all_teams.iter(), query).cloned().collect();

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
        let all_teams = self.storage.list().await?;
        Ok(filter_teams(all_teams.iter(), query).count())
    }

    async fn exists(&self, id: &TeamId) -> Result<bool, DomainError> {
        self.storage.exists(id).await
    }
}

fn filter_teams<'a>(
    teams: impl Iterator<Item = &'a Team>,
    query: &TeamQuery,
) -> impl Iterator<Item = &'a Team> {
    teams.filter(move |team| {
        if let Some(ref status) = query.status {
            if team.status().to_string() != *status {
                return false;
            }
        }

        true
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::storage::InMemoryStorage;

    fn create_repo() -> StorageTeamRepository {
        let storage = Arc::new(InMemoryStorage::<Team>::new());
        StorageTeamRepository::new(storage)
    }

    fn create_team(id: &str, name: &str) -> Team {
        Team::new(TeamId::new(id).unwrap(), name).unwrap()
    }

    #[tokio::test]
    async fn test_create_and_get() {
        let repo = create_repo();
        let team = create_team("team-1", "Team One");

        repo.create(team.clone()).await.unwrap();

        let retrieved = repo.get(team.id()).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name(), "Team One");
    }

    #[tokio::test]
    async fn test_create_duplicate() {
        let repo = create_repo();
        let team1 = create_team("team-1", "Team One");
        let team2 = create_team("team-1", "Team Two");

        repo.create(team1).await.unwrap();

        let result = repo.create(team2).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update() {
        let repo = create_repo();
        let team = create_team("team-1", "Team One");

        repo.create(team).await.unwrap();

        let id = TeamId::new("team-1").unwrap();
        let mut updated = repo.get(&id).await.unwrap().unwrap();
        updated.set_name("Updated Team").unwrap();

        repo.update(updated).await.unwrap();

        let retrieved = repo.get(&id).await.unwrap().unwrap();
        assert_eq!(retrieved.name(), "Updated Team");
    }

    #[tokio::test]
    async fn test_update_nonexistent() {
        let repo = create_repo();
        let team = create_team("nonexistent", "Team");

        let result = repo.update(team).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete() {
        let repo = create_repo();
        let team = create_team("team-1", "Team One");

        repo.create(team.clone()).await.unwrap();
        assert!(repo.exists(team.id()).await.unwrap());

        let deleted = repo.delete(team.id()).await.unwrap();
        assert!(deleted);
        assert!(!repo.exists(team.id()).await.unwrap());
    }

    #[tokio::test]
    async fn test_list() {
        let repo = create_repo();

        repo.create(create_team("team-a", "Team A")).await.unwrap();
        repo.create(create_team("team-b", "Team B")).await.unwrap();

        let teams = repo.list(&TeamQuery::new()).await.unwrap();
        assert_eq!(teams.len(), 2);
        // Should be sorted by name
        assert_eq!(teams[0].name(), "Team A");
        assert_eq!(teams[1].name(), "Team B");
    }

    #[tokio::test]
    async fn test_list_with_pagination() {
        let repo = create_repo();

        for i in 0..5 {
            repo.create(create_team(&format!("team-{}", i), &format!("Team {}", i)))
                .await
                .unwrap();
        }

        let query = TeamQuery::new().with_limit(2).with_offset(1);
        let teams = repo.list(&query).await.unwrap();
        assert_eq!(teams.len(), 2);
    }

    #[tokio::test]
    async fn test_count() {
        let repo = create_repo();

        repo.create(create_team("team-a", "Team A")).await.unwrap();
        repo.create(create_team("team-b", "Team B")).await.unwrap();

        let count = repo.count(&TeamQuery::new()).await.unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_exists() {
        let repo = create_repo();
        let team = create_team("team-1", "Team One");

        assert!(!repo.exists(team.id()).await.unwrap());

        repo.create(team.clone()).await.unwrap();

        assert!(repo.exists(team.id()).await.unwrap());
    }
}
