//! In-memory user repository implementation

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::domain::user::{User, UserId, UserRepository, UserStatus};
use crate::domain::DomainError;

/// In-memory implementation of UserRepository
#[derive(Debug)]
pub struct InMemoryUserRepository {
    users: Arc<RwLock<HashMap<String, User>>>,
    /// Index for username -> user ID lookup
    username_index: Arc<RwLock<HashMap<String, String>>>,
}

impl InMemoryUserRepository {
    /// Create a new empty repository
    pub fn new() -> Self {
        Self {
            users: Arc::new(RwLock::new(HashMap::new())),
            username_index: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a repository with initial users
    pub fn with_users(users: Vec<User>) -> Self {
        let mut users_map = HashMap::new();
        let mut username_map = HashMap::new();

        for user in users {
            let id = user.id().as_str().to_string();
            let username = user.username().to_string();
            username_map.insert(username, id.clone());
            users_map.insert(id, user);
        }

        Self {
            users: Arc::new(RwLock::new(users_map)),
            username_index: Arc::new(RwLock::new(username_map)),
        }
    }
}

impl Default for InMemoryUserRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl UserRepository for InMemoryUserRepository {
    async fn get(&self, id: &UserId) -> Result<Option<User>, DomainError> {
        let users = self.users.read().await;
        Ok(users.get(id.as_str()).cloned())
    }

    async fn get_by_username(&self, username: &str) -> Result<Option<User>, DomainError> {
        let username_index = self.username_index.read().await;

        if let Some(user_id) = username_index.get(username) {
            let users = self.users.read().await;
            return Ok(users.get(user_id).cloned());
        }

        Ok(None)
    }

    async fn create(&self, user: User) -> Result<User, DomainError> {
        let mut users = self.users.write().await;
        let mut username_index = self.username_index.write().await;

        let id = user.id().as_str().to_string();
        let username = user.username().to_string();

        if users.contains_key(&id) {
            return Err(DomainError::conflict(format!(
                "User with ID '{}' already exists",
                id
            )));
        }

        if username_index.contains_key(&username) {
            return Err(DomainError::conflict(format!(
                "Username '{}' already exists",
                username
            )));
        }

        username_index.insert(username, id.clone());
        users.insert(id, user.clone());

        Ok(user)
    }

    async fn update(&self, user: &User) -> Result<User, DomainError> {
        let mut users = self.users.write().await;
        let mut username_index = self.username_index.write().await;

        let id = user.id().as_str().to_string();

        if !users.contains_key(&id) {
            return Err(DomainError::not_found(format!("User '{}' not found", id)));
        }

        // Get the old user to check if username changed
        let old_user = users.get(&id).unwrap();
        let old_username = old_user.username().to_string();
        let new_username = user.username().to_string();

        // If username changed, check uniqueness and update index
        if old_username != new_username {
            if username_index.contains_key(&new_username) {
                return Err(DomainError::conflict(format!(
                    "Username '{}' already exists",
                    new_username
                )));
            }

            username_index.remove(&old_username);
            username_index.insert(new_username, id.clone());
        }

        users.insert(id, user.clone());

        Ok(user.clone())
    }

    async fn delete(&self, id: &UserId) -> Result<bool, DomainError> {
        let mut users = self.users.write().await;
        let mut username_index = self.username_index.write().await;

        if let Some(user) = users.remove(id.as_str()) {
            username_index.remove(user.username());
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn list(&self, status: Option<UserStatus>) -> Result<Vec<User>, DomainError> {
        let users = self.users.read().await;

        let result: Vec<User> = users
            .values()
            .filter(|u| {
                if let Some(s) = status {
                    u.status() == s
                } else {
                    true
                }
            })
            .cloned()
            .collect();

        Ok(result)
    }

    async fn count(&self, status: Option<UserStatus>) -> Result<usize, DomainError> {
        let users = self.users.read().await;

        let count = users
            .values()
            .filter(|u| {
                if let Some(s) = status {
                    u.status() == s
                } else {
                    true
                }
            })
            .count();

        Ok(count)
    }

    async fn record_login(&self, id: &UserId) -> Result<(), DomainError> {
        let mut users = self.users.write().await;

        if let Some(user) = users.get_mut(id.as_str()) {
            user.record_login();
            Ok(())
        } else {
            Err(DomainError::not_found(format!("User '{}' not found", id)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::team::{TeamId, TeamRole};

    fn admin_team() -> TeamId {
        TeamId::administrators()
    }

    fn create_test_user(id: &str, username: &str) -> User {
        let user_id = UserId::new(id).unwrap();
        User::new(user_id, username, "hashed_password", admin_team(), TeamRole::Member)
    }

    #[tokio::test]
    async fn test_create_and_get() {
        let repo = InMemoryUserRepository::new();
        let user = create_test_user("user-1", "testuser");

        repo.create(user.clone()).await.unwrap();

        let retrieved = repo.get(user.id()).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().username(), "testuser");
    }

    #[tokio::test]
    async fn test_get_by_username() {
        let repo = InMemoryUserRepository::new();
        let user = create_test_user("user-1", "testuser");

        repo.create(user).await.unwrap();

        let retrieved = repo.get_by_username("testuser").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id().as_str(), "user-1");

        let not_found = repo.get_by_username("nonexistent").await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_duplicate_id() {
        let repo = InMemoryUserRepository::new();
        let user1 = create_test_user("user-1", "user1");
        let user2 = create_test_user("user-1", "user2");

        repo.create(user1).await.unwrap();

        let result = repo.create(user2).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_duplicate_username() {
        let repo = InMemoryUserRepository::new();
        let user1 = create_test_user("user-1", "sameusername");
        let user2 = create_test_user("user-2", "sameusername");

        repo.create(user1).await.unwrap();

        let result = repo.create(user2).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update() {
        let repo = InMemoryUserRepository::new();
        let mut user = create_test_user("user-1", "testuser");

        repo.create(user.clone()).await.unwrap();

        user.set_username("newusername");
        repo.update(&user).await.unwrap();

        let retrieved = repo.get(user.id()).await.unwrap().unwrap();
        assert_eq!(retrieved.username(), "newusername");

        // Old username should not be found
        let old = repo.get_by_username("testuser").await.unwrap();
        assert!(old.is_none());

        // New username should be found
        let new = repo.get_by_username("newusername").await.unwrap();
        assert!(new.is_some());
    }

    #[tokio::test]
    async fn test_update_username_conflict() {
        let repo = InMemoryUserRepository::new();
        let user1 = create_test_user("user-1", "user1");
        let mut user2 = create_test_user("user-2", "user2");

        repo.create(user1).await.unwrap();
        repo.create(user2.clone()).await.unwrap();

        user2.set_username("user1"); // Try to change to existing username

        let result = repo.update(&user2).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete() {
        let repo = InMemoryUserRepository::new();
        let user = create_test_user("user-1", "testuser");

        repo.create(user.clone()).await.unwrap();

        let deleted = repo.delete(user.id()).await.unwrap();
        assert!(deleted);

        let retrieved = repo.get(user.id()).await.unwrap();
        assert!(retrieved.is_none());

        // Username should also be removed from index
        let by_username = repo.get_by_username("testuser").await.unwrap();
        assert!(by_username.is_none());
    }

    #[tokio::test]
    async fn test_list_and_count() {
        let repo = InMemoryUserRepository::new();

        repo.create(create_test_user("user-1", "user1")).await.unwrap();
        repo.create(create_test_user("user-2", "user2")).await.unwrap();

        let all = repo.list(None).await.unwrap();
        assert_eq!(all.len(), 2);

        let count = repo.count(None).await.unwrap();
        assert_eq!(count, 2);

        let active = repo.list(Some(UserStatus::Active)).await.unwrap();
        assert_eq!(active.len(), 2);
    }

    #[tokio::test]
    async fn test_record_login() {
        let repo = InMemoryUserRepository::new();
        let user = create_test_user("user-1", "testuser");

        repo.create(user.clone()).await.unwrap();

        let before = repo.get(user.id()).await.unwrap().unwrap();
        assert!(before.last_login_at().is_none());

        repo.record_login(user.id()).await.unwrap();

        let after = repo.get(user.id()).await.unwrap().unwrap();
        assert!(after.last_login_at().is_some());
    }

    #[tokio::test]
    async fn test_with_users() {
        let users = vec![
            create_test_user("user-1", "user1"),
            create_test_user("user-2", "user2"),
        ];

        let repo = InMemoryUserRepository::with_users(users);

        let count = repo.count(None).await.unwrap();
        assert_eq!(count, 2);

        let user1 = repo.get_by_username("user1").await.unwrap();
        assert!(user1.is_some());
    }
}
