//! User entity and related types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::validation::{validate_user_id, UserValidationError};
use crate::domain::team::{TeamId, TeamRole};

/// User identifier - alphanumeric + hyphens, max 50 characters
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct UserId(String);

impl UserId {
    /// Create a new UserId after validation
    pub fn new(id: impl Into<String>) -> Result<Self, UserValidationError> {
        let id = id.into();
        validate_user_id(&id)?;
        Ok(Self(id))
    }

    /// Get the inner string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for UserId {
    type Error = UserValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<UserId> for String {
    fn from(id: UserId) -> Self {
        id.0
    }
}

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Status of a user account
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum UserStatus {
    /// User is active and can log in
    #[default]
    Active,
    /// User is temporarily suspended
    Suspended,
}

impl UserStatus {
    /// Check if the user can log in
    pub fn can_login(&self) -> bool {
        matches!(self, Self::Active)
    }
}

/// User entity for authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// Unique identifier for the user
    id: UserId,
    /// Username for login
    username: String,
    /// Argon2 password hash - never exposed in serialization
    #[serde(skip_serializing)]
    password_hash: String,
    /// Current status of the user
    status: UserStatus,
    /// Team this user belongs to (required)
    team_id: TeamId,
    /// User's role within the team
    team_role: TeamRole,
    /// Creation timestamp
    created_at: DateTime<Utc>,
    /// Last update timestamp
    updated_at: DateTime<Utc>,
    /// Last login timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    last_login_at: Option<DateTime<Utc>>,
}

impl User {
    /// Create a new user
    pub fn new(
        id: UserId,
        username: impl Into<String>,
        password_hash: impl Into<String>,
        team_id: TeamId,
        team_role: TeamRole,
    ) -> Self {
        let now = Utc::now();

        Self {
            id,
            username: username.into(),
            password_hash: password_hash.into(),
            status: UserStatus::Active,
            team_id,
            team_role,
            created_at: now,
            updated_at: now,
            last_login_at: None,
        }
    }

    // Getters

    pub fn id(&self) -> &UserId {
        &self.id
    }

    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn password_hash(&self) -> &str {
        &self.password_hash
    }

    pub fn status(&self) -> UserStatus {
        self.status
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    pub fn last_login_at(&self) -> Option<DateTime<Utc>> {
        self.last_login_at
    }

    pub fn team_id(&self) -> &TeamId {
        &self.team_id
    }

    pub fn team_role(&self) -> TeamRole {
        self.team_role
    }

    // Status checks

    /// Check if the user is active and can log in
    pub fn is_active(&self) -> bool {
        self.status.can_login()
    }

    // Mutators

    /// Update the username
    pub fn set_username(&mut self, username: impl Into<String>) {
        self.username = username.into();
        self.touch();
    }

    /// Update the password hash
    pub fn set_password_hash(&mut self, password_hash: impl Into<String>) {
        self.password_hash = password_hash.into();
        self.touch();
    }

    /// Update the status
    pub fn set_status(&mut self, status: UserStatus) {
        self.status = status;
        self.touch();
    }

    /// Record a login
    pub fn record_login(&mut self) {
        self.last_login_at = Some(Utc::now());
    }

    /// Suspend the user
    pub fn suspend(&mut self) {
        self.status = UserStatus::Suspended;
        self.touch();
    }

    /// Activate a suspended user
    pub fn activate(&mut self) {
        if self.status == UserStatus::Suspended {
            self.status = UserStatus::Active;
            self.touch();
        }
    }

    /// Update the team assignment
    pub fn set_team(&mut self, team_id: TeamId, team_role: TeamRole) {
        self.team_id = team_id;
        self.team_role = team_role;
        self.touch();
    }

    /// Update just the team role
    pub fn set_team_role(&mut self, role: TeamRole) {
        self.team_role = role;
        self.touch();
    }

    fn touch(&mut self) {
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_user(id: &str, username: &str) -> User {
        let user_id = UserId::new(id).unwrap();
        let team_id = TeamId::administrators();
        User::new(user_id, username, "hashed_password", team_id, TeamRole::Member)
    }

    #[test]
    fn test_user_id_valid() {
        let id = UserId::new("admin").unwrap();
        assert_eq!(id.as_str(), "admin");
    }

    #[test]
    fn test_user_id_with_hyphens() {
        let id = UserId::new("user-123").unwrap();
        assert_eq!(id.as_str(), "user-123");
    }

    #[test]
    fn test_user_id_invalid() {
        assert!(UserId::new("").is_err());
        assert!(UserId::new("-user").is_err());
        assert!(UserId::new("user-").is_err());
    }

    #[test]
    fn test_user_status() {
        assert!(UserStatus::Active.can_login());
        assert!(!UserStatus::Suspended.can_login());
    }

    #[test]
    fn test_user_creation() {
        let user = create_test_user("admin", "admin");

        assert_eq!(user.username(), "admin");
        assert_eq!(user.password_hash(), "hashed_password");
        assert!(user.is_active());
        assert!(user.last_login_at().is_none());
        assert_eq!(user.team_id().as_str(), "administrators");
        assert_eq!(user.team_role(), TeamRole::Member);
    }

    #[test]
    fn test_user_with_team() {
        let user_id = UserId::new("team-user").unwrap();
        let team_id = TeamId::new("my-team").unwrap();
        let user = User::new(user_id, "teamuser", "hash", team_id, TeamRole::Admin);

        assert_eq!(user.team_id().as_str(), "my-team");
        assert_eq!(user.team_role(), TeamRole::Admin);
    }

    #[test]
    fn test_user_status_changes() {
        let mut user = create_test_user("admin", "admin");

        assert!(user.is_active());

        user.suspend();
        assert!(!user.is_active());
        assert_eq!(user.status(), UserStatus::Suspended);

        user.activate();
        assert!(user.is_active());
        assert_eq!(user.status(), UserStatus::Active);
    }

    #[test]
    fn test_user_record_login() {
        let mut user = create_test_user("admin", "admin");

        assert!(user.last_login_at().is_none());

        user.record_login();
        assert!(user.last_login_at().is_some());
    }

    #[test]
    fn test_user_update_password() {
        let mut user = create_test_user("admin", "admin");
        let original_updated = user.updated_at();

        // Small delay to ensure timestamp differs
        std::thread::sleep(std::time::Duration::from_millis(10));

        user.set_password_hash("new_hash");
        assert_eq!(user.password_hash(), "new_hash");
        assert!(user.updated_at() > original_updated);
    }

    #[test]
    fn test_user_serialization_excludes_password() {
        let user = create_test_user("admin", "admin");

        let json = serde_json::to_string(&user).unwrap();
        assert!(!json.contains("hashed_password"));
        assert!(!json.contains("password_hash"));
    }

    #[test]
    fn test_user_set_team() {
        let mut user = create_test_user("admin", "admin");
        let new_team_id = TeamId::new("new-team").unwrap();

        user.set_team(new_team_id, TeamRole::Owner);
        assert_eq!(user.team_id().as_str(), "new-team");
        assert_eq!(user.team_role(), TeamRole::Owner);
    }

    #[test]
    fn test_user_set_team_role() {
        let mut user = create_test_user("admin", "admin");

        user.set_team_role(TeamRole::Admin);
        assert_eq!(user.team_role(), TeamRole::Admin);
    }
}
