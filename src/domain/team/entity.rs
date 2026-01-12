//! Team entity and related types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::validation::{validate_team_id, validate_team_name, TeamValidationError};
use crate::domain::storage::{StorageEntity, StorageKey};

/// Team identifier - alphanumeric + hyphens, max 50 characters
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct TeamId(String);

impl TeamId {
    /// Create a new TeamId after validation
    pub fn new(id: impl Into<String>) -> Result<Self, TeamValidationError> {
        let id = id.into();
        validate_team_id(&id)?;
        Ok(Self(id))
    }

    /// Get the inner string value
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The ID of the built-in Administrators team
    pub const ADMINISTRATORS: &'static str = "administrators";

    /// Create the Administrators team ID
    pub fn administrators() -> Self {
        Self(Self::ADMINISTRATORS.to_string())
    }
}

impl TryFrom<String> for TeamId {
    type Error = TeamValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<TeamId> for String {
    fn from(id: TeamId) -> Self {
        id.0
    }
}

impl std::fmt::Display for TeamId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl StorageKey for TeamId {
    fn as_str(&self) -> &str {
        &self.0
    }
}

/// Status of a team
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TeamStatus {
    /// Team is active
    #[default]
    Active,
    /// Team is suspended
    Suspended,
}

impl TeamStatus {
    /// Check if the team is active
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Active)
    }
}

impl std::fmt::Display for TeamStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Suspended => write!(f, "suspended"),
        }
    }
}

/// Role of a user within a team
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TeamRole {
    /// Team owner - full control including team deletion
    Owner,
    /// Team admin - can manage members and resources
    Admin,
    /// Regular team member
    #[default]
    Member,
}

impl TeamRole {
    /// Check if this role can manage team members
    pub fn can_manage_members(&self) -> bool {
        matches!(self, Self::Owner | Self::Admin)
    }

    /// Check if this role can manage team resources
    pub fn can_manage_resources(&self) -> bool {
        matches!(self, Self::Owner | Self::Admin)
    }

    /// Check if this role can delete the team
    pub fn can_delete_team(&self) -> bool {
        matches!(self, Self::Owner)
    }

    /// Check if this role has higher or equal privilege than another
    pub fn has_privilege_over(&self, other: &TeamRole) -> bool {
        match (self, other) {
            (Self::Owner, _) => true,
            (Self::Admin, Self::Admin) | (Self::Admin, Self::Member) => true,
            (Self::Member, Self::Member) => true,
            _ => false,
        }
    }
}

impl std::fmt::Display for TeamRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Owner => write!(f, "owner"),
            Self::Admin => write!(f, "admin"),
            Self::Member => write!(f, "member"),
        }
    }
}

/// Team entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Team {
    /// Unique identifier
    id: TeamId,
    /// Display name
    name: String,
    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    /// Current status
    status: TeamStatus,
    /// Creation timestamp
    created_at: DateTime<Utc>,
    /// Last update timestamp
    updated_at: DateTime<Utc>,
}

impl Team {
    /// Create a new team
    pub fn new(id: TeamId, name: impl Into<String>) -> Result<Self, TeamValidationError> {
        let name = name.into();
        validate_team_name(&name)?;
        let now = Utc::now();

        Ok(Self {
            id,
            name,
            description: None,
            status: TeamStatus::Active,
            created_at: now,
            updated_at: now,
        })
    }

    /// Create the built-in Administrators team
    pub fn administrators() -> Self {
        let now = Utc::now();

        Self {
            id: TeamId::administrators(),
            name: "Administrators".to_string(),
            description: Some("Built-in administrators team".to_string()),
            status: TeamStatus::Active,
            created_at: now,
            updated_at: now,
        }
    }

    /// Set description (builder pattern)
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    // Getters

    pub fn id(&self) -> &TeamId {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn status(&self) -> TeamStatus {
        self.status
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    /// Check if this is the built-in Administrators team
    pub fn is_administrators(&self) -> bool {
        self.id.as_str() == TeamId::ADMINISTRATORS
    }

    // Mutators

    /// Update the name
    pub fn set_name(&mut self, name: impl Into<String>) -> Result<(), TeamValidationError> {
        let name = name.into();
        validate_team_name(&name)?;
        self.name = name;
        self.touch();
        Ok(())
    }

    /// Update the description
    pub fn set_description(&mut self, description: Option<String>) {
        self.description = description;
        self.touch();
    }

    /// Suspend the team
    pub fn suspend(&mut self) {
        self.status = TeamStatus::Suspended;
        self.touch();
    }

    /// Activate a suspended team
    pub fn activate(&mut self) {
        if self.status == TeamStatus::Suspended {
            self.status = TeamStatus::Active;
            self.touch();
        }
    }

    fn touch(&mut self) {
        self.updated_at = Utc::now();
    }
}

impl StorageEntity for Team {
    type Key = TeamId;

    fn key(&self) -> &Self::Key {
        &self.id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_team_id_valid() {
        let id = TeamId::new("my-team").unwrap();
        assert_eq!(id.as_str(), "my-team");
    }

    #[test]
    fn test_team_id_with_numbers() {
        let id = TeamId::new("team-123").unwrap();
        assert_eq!(id.as_str(), "team-123");
    }

    #[test]
    fn test_team_id_invalid() {
        assert!(TeamId::new("").is_err());
        assert!(TeamId::new("-team").is_err());
        assert!(TeamId::new("team-").is_err());
        assert!(TeamId::new("team_name").is_err());
    }

    #[test]
    fn test_team_id_administrators() {
        let id = TeamId::administrators();
        assert_eq!(id.as_str(), "administrators");
    }

    #[test]
    fn test_team_status() {
        assert!(TeamStatus::Active.is_active());
        assert!(!TeamStatus::Suspended.is_active());
    }

    #[test]
    fn test_team_role_privileges() {
        assert!(TeamRole::Owner.can_manage_members());
        assert!(TeamRole::Owner.can_manage_resources());
        assert!(TeamRole::Owner.can_delete_team());

        assert!(TeamRole::Admin.can_manage_members());
        assert!(TeamRole::Admin.can_manage_resources());
        assert!(!TeamRole::Admin.can_delete_team());

        assert!(!TeamRole::Member.can_manage_members());
        assert!(!TeamRole::Member.can_manage_resources());
        assert!(!TeamRole::Member.can_delete_team());
    }

    #[test]
    fn test_team_role_privilege_over() {
        assert!(TeamRole::Owner.has_privilege_over(&TeamRole::Owner));
        assert!(TeamRole::Owner.has_privilege_over(&TeamRole::Admin));
        assert!(TeamRole::Owner.has_privilege_over(&TeamRole::Member));

        assert!(!TeamRole::Admin.has_privilege_over(&TeamRole::Owner));
        assert!(TeamRole::Admin.has_privilege_over(&TeamRole::Admin));
        assert!(TeamRole::Admin.has_privilege_over(&TeamRole::Member));

        assert!(!TeamRole::Member.has_privilege_over(&TeamRole::Owner));
        assert!(!TeamRole::Member.has_privilege_over(&TeamRole::Admin));
        assert!(TeamRole::Member.has_privilege_over(&TeamRole::Member));
    }

    #[test]
    fn test_team_creation() {
        let id = TeamId::new("my-team").unwrap();
        let team = Team::new(id, "My Team").unwrap();

        assert_eq!(team.name(), "My Team");
        assert!(team.description().is_none());
        assert!(team.status().is_active());
    }

    #[test]
    fn test_team_with_description() {
        let id = TeamId::new("my-team").unwrap();
        let team = Team::new(id, "My Team")
            .unwrap()
            .with_description("A test team");

        assert_eq!(team.description(), Some("A test team"));
    }

    #[test]
    fn test_team_administrators() {
        let team = Team::administrators();

        assert_eq!(team.id().as_str(), "administrators");
        assert_eq!(team.name(), "Administrators");
        assert!(team.is_administrators());
    }

    #[test]
    fn test_team_status_changes() {
        let id = TeamId::new("my-team").unwrap();
        let mut team = Team::new(id, "My Team").unwrap();

        assert!(team.status().is_active());

        team.suspend();
        assert!(!team.status().is_active());
        assert_eq!(team.status(), TeamStatus::Suspended);

        team.activate();
        assert!(team.status().is_active());
        assert_eq!(team.status(), TeamStatus::Active);
    }

    #[test]
    fn test_team_update_name() {
        let id = TeamId::new("my-team").unwrap();
        let mut team = Team::new(id, "My Team").unwrap();
        let original_updated = team.updated_at();

        std::thread::sleep(std::time::Duration::from_millis(10));

        team.set_name("New Name").unwrap();
        assert_eq!(team.name(), "New Name");
        assert!(team.updated_at() > original_updated);
    }

    #[test]
    fn test_team_invalid_name() {
        let id = TeamId::new("my-team").unwrap();
        assert!(Team::new(id, "").is_err());
    }
}
