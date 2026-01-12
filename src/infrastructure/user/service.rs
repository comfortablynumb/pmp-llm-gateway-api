//! User service for authentication and user management

use std::sync::Arc;

use crate::domain::team::{TeamId, TeamRole};
use crate::domain::user::{
    validate_password, validate_username, User, UserId, UserRepository, UserStatus,
};
use crate::domain::DomainError;

use super::password::PasswordHasher;

/// Request for creating a new user
#[derive(Debug, Clone)]
pub struct CreateUserRequest {
    pub id: String,
    pub username: String,
    pub password: String,
    pub team_id: TeamId,
    pub team_role: TeamRole,
}

/// Request for updating a user's password
#[derive(Debug, Clone)]
pub struct UpdatePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

/// User service for authentication and management
#[derive(Debug)]
pub struct UserService<R: UserRepository, H: PasswordHasher> {
    repository: Arc<R>,
    hasher: Arc<H>,
}

impl<R: UserRepository, H: PasswordHasher> UserService<R, H> {
    /// Create a new user service
    pub fn new(repository: Arc<R>, hasher: Arc<H>) -> Self {
        Self { repository, hasher }
    }

    /// Create a new user
    pub async fn create(&self, request: CreateUserRequest) -> Result<User, DomainError> {
        // Validate username
        validate_username(&request.username).map_err(|e| DomainError::validation(e.to_string()))?;

        // Validate password
        validate_password(&request.password).map_err(|e| DomainError::validation(e.to_string()))?;

        // Parse user ID
        let user_id = UserId::new(&request.id).map_err(|e| DomainError::invalid_id(e.to_string()))?;

        // Check if username already exists
        if self.repository.username_exists(&request.username).await? {
            return Err(DomainError::conflict(format!(
                "Username '{}' already exists",
                request.username
            )));
        }

        // Hash the password
        let password_hash = self.hasher.hash(&request.password)?;

        // Create the user
        let user = User::new(
            user_id,
            &request.username,
            password_hash,
            request.team_id,
            request.team_role,
        );

        self.repository.create(user).await
    }

    /// Authenticate a user with username and password
    pub async fn authenticate(
        &self,
        username: &str,
        password: &str,
    ) -> Result<Option<User>, DomainError> {
        // Look up user by username
        let user = match self.repository.get_by_username(username).await? {
            Some(u) => u,
            None => return Ok(None),
        };

        // Check if user is active
        if !user.is_active() {
            return Ok(None);
        }

        // Verify password
        if !self.hasher.verify(password, user.password_hash()) {
            return Ok(None);
        }

        // Record login
        self.repository.record_login(user.id()).await?;

        // Re-fetch user to get updated last_login_at
        self.repository.get(user.id()).await
    }

    /// Get a user by ID
    pub async fn get(&self, id: &str) -> Result<Option<User>, DomainError> {
        let user_id = UserId::new(id).map_err(|e| DomainError::invalid_id(e.to_string()))?;
        self.repository.get(&user_id).await
    }

    /// Get a user by username
    pub async fn get_by_username(&self, username: &str) -> Result<Option<User>, DomainError> {
        self.repository.get_by_username(username).await
    }

    /// List all users
    pub async fn list(&self, status: Option<UserStatus>) -> Result<Vec<User>, DomainError> {
        self.repository.list(status).await
    }

    /// Count users
    pub async fn count(&self, status: Option<UserStatus>) -> Result<usize, DomainError> {
        self.repository.count(status).await
    }

    /// Update a user's password
    pub async fn update_password(
        &self,
        id: &str,
        request: UpdatePasswordRequest,
    ) -> Result<User, DomainError> {
        let user_id = UserId::new(id).map_err(|e| DomainError::invalid_id(e.to_string()))?;

        // Get user
        let mut user = self
            .repository
            .get(&user_id)
            .await?
            .ok_or_else(|| DomainError::not_found(format!("User '{}' not found", id)))?;

        // Verify current password
        if !self.hasher.verify(&request.current_password, user.password_hash()) {
            return Err(DomainError::validation("Current password is incorrect"));
        }

        // Validate new password
        validate_password(&request.new_password)
            .map_err(|e| DomainError::validation(e.to_string()))?;

        // Hash new password
        let new_hash = self.hasher.hash(&request.new_password)?;
        user.set_password_hash(new_hash);

        self.repository.update(&user).await
    }

    /// Suspend a user
    pub async fn suspend(&self, id: &str) -> Result<User, DomainError> {
        let user_id = UserId::new(id).map_err(|e| DomainError::invalid_id(e.to_string()))?;

        let mut user = self
            .repository
            .get(&user_id)
            .await?
            .ok_or_else(|| DomainError::not_found(format!("User '{}' not found", id)))?;

        user.suspend();

        self.repository.update(&user).await
    }

    /// Activate a suspended user
    pub async fn activate(&self, id: &str) -> Result<User, DomainError> {
        let user_id = UserId::new(id).map_err(|e| DomainError::invalid_id(e.to_string()))?;

        let mut user = self
            .repository
            .get(&user_id)
            .await?
            .ok_or_else(|| DomainError::not_found(format!("User '{}' not found", id)))?;

        user.activate();

        self.repository.update(&user).await
    }

    /// Delete a user
    pub async fn delete(&self, id: &str) -> Result<bool, DomainError> {
        let user_id = UserId::new(id).map_err(|e| DomainError::invalid_id(e.to_string()))?;
        self.repository.delete(&user_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::user::password::Argon2Hasher;
    use crate::infrastructure::user::repository::InMemoryUserRepository;

    fn create_service() -> UserService<InMemoryUserRepository, Argon2Hasher> {
        let repository = Arc::new(InMemoryUserRepository::new());
        let hasher = Arc::new(Argon2Hasher::new());
        UserService::new(repository, hasher)
    }

    fn admin_team() -> TeamId {
        TeamId::administrators()
    }

    fn make_request(id: &str, username: &str, password: &str) -> CreateUserRequest {
        CreateUserRequest {
            id: id.to_string(),
            username: username.to_string(),
            password: password.to_string(),
            team_id: admin_team(),
            team_role: TeamRole::Member,
        }
    }

    #[tokio::test]
    async fn test_create_user() {
        let service = create_service();

        let request = make_request("user-1", "testuser", "secure_password123");

        let user = service.create(request).await.unwrap();
        assert_eq!(user.username(), "testuser");
        assert!(user.is_active());
        assert_eq!(user.team_id(), &admin_team());
        assert_eq!(user.team_role(), TeamRole::Member);
    }

    #[tokio::test]
    async fn test_create_user_invalid_username() {
        let service = create_service();

        let request = make_request("user-1", "ab", "secure_password123"); // Too short

        let result = service.create(request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_user_invalid_password() {
        let service = create_service();

        let request = make_request("user-1", "testuser", "short"); // Too short

        let result = service.create(request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_duplicate_username() {
        let service = create_service();

        let request1 = make_request("user-1", "testuser", "secure_password123");
        let request2 = make_request("user-2", "testuser", "secure_password456"); // Same username

        service.create(request1).await.unwrap();

        let result = service.create(request2).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_authenticate_success() {
        let service = create_service();

        let request = make_request("user-1", "testuser", "secure_password123");

        service.create(request).await.unwrap();

        let user = service
            .authenticate("testuser", "secure_password123")
            .await
            .unwrap();

        assert!(user.is_some());
        assert!(user.unwrap().last_login_at().is_some());
    }

    #[tokio::test]
    async fn test_authenticate_wrong_password() {
        let service = create_service();

        let request = make_request("user-1", "testuser", "secure_password123");

        service.create(request).await.unwrap();

        let user = service
            .authenticate("testuser", "wrong_password")
            .await
            .unwrap();

        assert!(user.is_none());
    }

    #[tokio::test]
    async fn test_authenticate_nonexistent_user() {
        let service = create_service();

        let user = service
            .authenticate("nonexistent", "password")
            .await
            .unwrap();

        assert!(user.is_none());
    }

    #[tokio::test]
    async fn test_authenticate_suspended_user() {
        let service = create_service();

        let request = make_request("user-1", "testuser", "secure_password123");

        service.create(request).await.unwrap();
        service.suspend("user-1").await.unwrap();

        let user = service
            .authenticate("testuser", "secure_password123")
            .await
            .unwrap();

        assert!(user.is_none());
    }

    #[tokio::test]
    async fn test_update_password() {
        let service = create_service();

        let request = make_request("user-1", "testuser", "old_password123");

        service.create(request).await.unwrap();

        let update_request = UpdatePasswordRequest {
            current_password: "old_password123".to_string(),
            new_password: "new_password456".to_string(),
        };

        service.update_password("user-1", update_request).await.unwrap();

        // Old password should fail
        let old_auth = service
            .authenticate("testuser", "old_password123")
            .await
            .unwrap();
        assert!(old_auth.is_none());

        // New password should work
        let new_auth = service
            .authenticate("testuser", "new_password456")
            .await
            .unwrap();
        assert!(new_auth.is_some());
    }

    #[tokio::test]
    async fn test_update_password_wrong_current() {
        let service = create_service();

        let request = make_request("user-1", "testuser", "current_password");

        service.create(request).await.unwrap();

        let update_request = UpdatePasswordRequest {
            current_password: "wrong_current".to_string(),
            new_password: "new_password456".to_string(),
        };

        let result = service.update_password("user-1", update_request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_suspend_and_activate() {
        let service = create_service();

        let request = make_request("user-1", "testuser", "secure_password123");

        service.create(request).await.unwrap();

        // Suspend
        let suspended = service.suspend("user-1").await.unwrap();
        assert_eq!(suspended.status(), UserStatus::Suspended);

        // Activate
        let activated = service.activate("user-1").await.unwrap();
        assert_eq!(activated.status(), UserStatus::Active);
    }

    #[tokio::test]
    async fn test_list_and_count() {
        let service = create_service();

        service
            .create(make_request("user-1", "user1", "password123"))
            .await
            .unwrap();

        service
            .create(make_request("user-2", "user2", "password123"))
            .await
            .unwrap();

        let all = service.list(None).await.unwrap();
        assert_eq!(all.len(), 2);

        let count = service.count(None).await.unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_delete() {
        let service = create_service();

        service
            .create(make_request("user-1", "testuser", "password123"))
            .await
            .unwrap();

        let deleted = service.delete("user-1").await.unwrap();
        assert!(deleted);

        let user = service.get("user-1").await.unwrap();
        assert!(user.is_none());
    }
}
