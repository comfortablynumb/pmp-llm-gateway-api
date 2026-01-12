//! User repository trait

use async_trait::async_trait;
use std::fmt::Debug;

use super::entity::{User, UserId, UserStatus};
use crate::domain::DomainError;

/// Repository trait for user storage
#[async_trait]
pub trait UserRepository: Send + Sync + Debug {
    /// Get a user by their ID
    async fn get(&self, id: &UserId) -> Result<Option<User>, DomainError>;

    /// Get a user by their username (for login)
    async fn get_by_username(&self, username: &str) -> Result<Option<User>, DomainError>;

    /// Create a new user
    async fn create(&self, user: User) -> Result<User, DomainError>;

    /// Update an existing user
    async fn update(&self, user: &User) -> Result<User, DomainError>;

    /// Delete a user
    async fn delete(&self, id: &UserId) -> Result<bool, DomainError>;

    /// List all users (optionally filtered by status)
    async fn list(&self, status: Option<UserStatus>) -> Result<Vec<User>, DomainError>;

    /// Count users (optionally filtered by status)
    async fn count(&self, status: Option<UserStatus>) -> Result<usize, DomainError>;

    /// Check if a user ID exists
    async fn exists(&self, id: &UserId) -> Result<bool, DomainError> {
        Ok(self.get(id).await?.is_some())
    }

    /// Check if a username exists
    async fn username_exists(&self, username: &str) -> Result<bool, DomainError> {
        Ok(self.get_by_username(username).await?.is_some())
    }

    /// Record a login for a user
    async fn record_login(&self, id: &UserId) -> Result<(), DomainError>;
}

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    /// Mock user repository for testing
    #[derive(Debug, Default)]
    pub struct MockUserRepository {
        users: Arc<RwLock<HashMap<String, User>>>,
        should_fail: Arc<RwLock<bool>>,
    }

    impl MockUserRepository {
        /// Create a new mock repository
        pub fn new() -> Self {
            Self::default()
        }

        /// Set whether operations should fail
        pub async fn set_should_fail(&self, fail: bool) {
            *self.should_fail.write().await = fail;
        }

        async fn check_should_fail(&self) -> Result<(), DomainError> {
            if *self.should_fail.read().await {
                return Err(DomainError::storage("Mock repository configured to fail"));
            }
            Ok(())
        }
    }

    #[async_trait]
    impl UserRepository for MockUserRepository {
        async fn get(&self, id: &UserId) -> Result<Option<User>, DomainError> {
            self.check_should_fail().await?;
            let users = self.users.read().await;
            Ok(users.get(id.as_str()).cloned())
        }

        async fn get_by_username(&self, username: &str) -> Result<Option<User>, DomainError> {
            self.check_should_fail().await?;
            let users = self.users.read().await;
            Ok(users.values().find(|u| u.username() == username).cloned())
        }

        async fn create(&self, user: User) -> Result<User, DomainError> {
            self.check_should_fail().await?;
            let mut users = self.users.write().await;
            let id = user.id().as_str().to_string();

            if users.contains_key(&id) {
                return Err(DomainError::conflict(format!(
                    "User with ID '{}' already exists",
                    id
                )));
            }

            // Check username uniqueness
            if users.values().any(|u| u.username() == user.username()) {
                return Err(DomainError::conflict(format!(
                    "Username '{}' already exists",
                    user.username()
                )));
            }

            users.insert(id, user.clone());
            Ok(user)
        }

        async fn update(&self, user: &User) -> Result<User, DomainError> {
            self.check_should_fail().await?;
            let mut users = self.users.write().await;
            let id = user.id().as_str().to_string();

            if !users.contains_key(&id) {
                return Err(DomainError::not_found(format!(
                    "User '{}' not found",
                    id
                )));
            }

            // Check username uniqueness (exclude current user)
            let username_taken = users
                .values()
                .any(|u| u.username() == user.username() && u.id().as_str() != id);

            if username_taken {
                return Err(DomainError::conflict(format!(
                    "Username '{}' already exists",
                    user.username()
                )));
            }

            users.insert(id, user.clone());
            Ok(user.clone())
        }

        async fn delete(&self, id: &UserId) -> Result<bool, DomainError> {
            self.check_should_fail().await?;
            let mut users = self.users.write().await;
            Ok(users.remove(id.as_str()).is_some())
        }

        async fn list(&self, status: Option<UserStatus>) -> Result<Vec<User>, DomainError> {
            self.check_should_fail().await?;
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
            self.check_should_fail().await?;
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
            self.check_should_fail().await?;
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
            let repo = MockUserRepository::new();
            let user = create_test_user("user-1", "testuser");

            repo.create(user.clone()).await.unwrap();

            let retrieved = repo.get(user.id()).await.unwrap();
            assert!(retrieved.is_some());
            assert_eq!(retrieved.unwrap().username(), user.username());
        }

        #[tokio::test]
        async fn test_get_by_username() {
            let repo = MockUserRepository::new();
            let user = create_test_user("user-1", "testuser");

            repo.create(user.clone()).await.unwrap();

            let retrieved = repo.get_by_username("testuser").await.unwrap();
            assert!(retrieved.is_some());
            assert_eq!(retrieved.unwrap().id().as_str(), "user-1");
        }

        #[tokio::test]
        async fn test_username_uniqueness() {
            let repo = MockUserRepository::new();
            let user1 = create_test_user("user-1", "testuser");
            let user2 = create_test_user("user-2", "testuser");

            repo.create(user1).await.unwrap();

            let result = repo.create(user2).await;
            assert!(result.is_err());
        }

        #[tokio::test]
        async fn test_update() {
            let repo = MockUserRepository::new();
            let mut user = create_test_user("user-1", "testuser");

            repo.create(user.clone()).await.unwrap();

            user.set_username("newusername");
            repo.update(&user).await.unwrap();

            let retrieved = repo.get(user.id()).await.unwrap().unwrap();
            assert_eq!(retrieved.username(), "newusername");
        }

        #[tokio::test]
        async fn test_delete() {
            let repo = MockUserRepository::new();
            let user = create_test_user("user-1", "testuser");

            repo.create(user.clone()).await.unwrap();

            let deleted = repo.delete(user.id()).await.unwrap();
            assert!(deleted);

            let retrieved = repo.get(user.id()).await.unwrap();
            assert!(retrieved.is_none());
        }

        #[tokio::test]
        async fn test_list() {
            let repo = MockUserRepository::new();

            repo.create(create_test_user("user-1", "user1")).await.unwrap();
            repo.create(create_test_user("user-2", "user2")).await.unwrap();

            let all = repo.list(None).await.unwrap();
            assert_eq!(all.len(), 2);

            let active = repo.list(Some(UserStatus::Active)).await.unwrap();
            assert_eq!(active.len(), 2);
        }

        #[tokio::test]
        async fn test_count() {
            let repo = MockUserRepository::new();

            repo.create(create_test_user("user-1", "user1")).await.unwrap();
            repo.create(create_test_user("user-2", "user2")).await.unwrap();

            let count = repo.count(None).await.unwrap();
            assert_eq!(count, 2);
        }

        #[tokio::test]
        async fn test_record_login() {
            let repo = MockUserRepository::new();
            let user = create_test_user("user-1", "testuser");

            repo.create(user.clone()).await.unwrap();

            repo.record_login(user.id()).await.unwrap();

            let retrieved = repo.get(user.id()).await.unwrap().unwrap();
            assert!(retrieved.last_login_at().is_some());
        }
    }
}
