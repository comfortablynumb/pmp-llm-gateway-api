//! PostgreSQL user repository implementation

use async_trait::async_trait;
use sqlx::{PgPool, Row};

use crate::domain::team::{TeamId, TeamRole};
use crate::domain::user::{User, UserId, UserRepository, UserStatus};
use crate::domain::DomainError;

/// PostgreSQL implementation of UserRepository
#[derive(Debug, Clone)]
pub struct PostgresUserRepository {
    pool: PgPool,
}

impl PostgresUserRepository {
    /// Create a new repository with the given connection pool
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserRepository for PostgresUserRepository {
    async fn get(&self, id: &UserId) -> Result<Option<User>, DomainError> {
        let row = sqlx::query(
            r#"
            SELECT id, username, password_hash, status, team_id, team_role,
                   created_at, updated_at, last_login_at
            FROM users
            WHERE id = $1
            "#,
        )
        .bind(id.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::storage(format!("Failed to get user: {}", e)))?;

        match row {
            Some(row) => Ok(Some(row_to_user(&row)?)),
            None => Ok(None),
        }
    }

    async fn get_by_username(&self, username: &str) -> Result<Option<User>, DomainError> {
        let row = sqlx::query(
            r#"
            SELECT id, username, password_hash, status, team_id, team_role,
                   created_at, updated_at, last_login_at
            FROM users
            WHERE username = $1
            "#,
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::storage(format!("Failed to get user by username: {}", e)))?;

        match row {
            Some(row) => Ok(Some(row_to_user(&row)?)),
            None => Ok(None),
        }
    }

    async fn create(&self, user: User) -> Result<User, DomainError> {
        sqlx::query(
            r#"
            INSERT INTO users (id, username, password_hash, status, team_id, team_role,
                             created_at, updated_at, last_login_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(user.id().as_str())
        .bind(user.username())
        .bind(user.password_hash())
        .bind(status_to_str(user.status()))
        .bind(user.team_id().as_str())
        .bind(role_to_str(user.team_role()))
        .bind(user.created_at())
        .bind(user.updated_at())
        .bind(user.last_login_at())
        .execute(&self.pool)
        .await
        .map_err(|e| {
            let msg = e.to_string();

            if msg.contains("duplicate key") || msg.contains("unique constraint") {
                if msg.contains("username") {
                    DomainError::conflict(format!(
                        "Username '{}' already exists",
                        user.username()
                    ))
                } else {
                    DomainError::conflict(format!(
                        "User with ID '{}' already exists",
                        user.id().as_str()
                    ))
                }
            } else {
                DomainError::storage(format!("Failed to create user: {}", e))
            }
        })?;

        Ok(user)
    }

    async fn update(&self, user: &User) -> Result<User, DomainError> {
        let result = sqlx::query(
            r#"
            UPDATE users
            SET username = $2, password_hash = $3, status = $4, team_id = $5,
                team_role = $6, updated_at = $7, last_login_at = $8
            WHERE id = $1
            "#,
        )
        .bind(user.id().as_str())
        .bind(user.username())
        .bind(user.password_hash())
        .bind(status_to_str(user.status()))
        .bind(user.team_id().as_str())
        .bind(role_to_str(user.team_role()))
        .bind(user.updated_at())
        .bind(user.last_login_at())
        .execute(&self.pool)
        .await
        .map_err(|e| {
            let msg = e.to_string();

            if msg.contains("duplicate key") || msg.contains("unique constraint") {
                DomainError::conflict(format!(
                    "Username '{}' already exists",
                    user.username()
                ))
            } else {
                DomainError::storage(format!("Failed to update user: {}", e))
            }
        })?;

        if result.rows_affected() == 0 {
            return Err(DomainError::not_found(format!(
                "User '{}' not found",
                user.id().as_str()
            )));
        }

        Ok(user.clone())
    }

    async fn delete(&self, id: &UserId) -> Result<bool, DomainError> {
        let result = sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(id.as_str())
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::storage(format!("Failed to delete user: {}", e)))?;

        Ok(result.rows_affected() > 0)
    }

    async fn list(&self, status: Option<UserStatus>) -> Result<Vec<User>, DomainError> {
        let rows = match status {
            Some(s) => {
                sqlx::query(
                    r#"
                    SELECT id, username, password_hash, status, team_id, team_role,
                           created_at, updated_at, last_login_at
                    FROM users
                    WHERE status = $1
                    ORDER BY created_at
                    "#,
                )
                .bind(status_to_str(s))
                .fetch_all(&self.pool)
                .await
            }
            None => {
                sqlx::query(
                    r#"
                    SELECT id, username, password_hash, status, team_id, team_role,
                           created_at, updated_at, last_login_at
                    FROM users
                    ORDER BY created_at
                    "#,
                )
                .fetch_all(&self.pool)
                .await
            }
        }
        .map_err(|e| DomainError::storage(format!("Failed to list users: {}", e)))?;

        let mut users = Vec::with_capacity(rows.len());

        for row in rows {
            users.push(row_to_user(&row)?);
        }

        Ok(users)
    }

    async fn count(&self, status: Option<UserStatus>) -> Result<usize, DomainError> {
        let count: i64 = match status {
            Some(s) => {
                sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE status = $1")
                    .bind(status_to_str(s))
                    .fetch_one(&self.pool)
                    .await
            }
            None => {
                sqlx::query_scalar("SELECT COUNT(*) FROM users")
                    .fetch_one(&self.pool)
                    .await
            }
        }
        .map_err(|e| DomainError::storage(format!("Failed to count users: {}", e)))?;

        Ok(count as usize)
    }

    async fn record_login(&self, id: &UserId) -> Result<(), DomainError> {
        let result = sqlx::query(
            "UPDATE users SET last_login_at = NOW() WHERE id = $1",
        )
        .bind(id.as_str())
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::storage(format!("Failed to record login: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(DomainError::not_found(format!(
                "User '{}' not found",
                id.as_str()
            )));
        }

        Ok(())
    }
}

fn row_to_user(row: &sqlx::postgres::PgRow) -> Result<User, DomainError> {
    let id: String = row.get("id");
    let username: String = row.get("username");
    let password_hash: String = row.get("password_hash");
    let status: String = row.get("status");
    let team_id: String = row.get("team_id");
    let team_role: String = row.get("team_role");
    let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");
    let updated_at: chrono::DateTime<chrono::Utc> = row.get("updated_at");
    let last_login_at: Option<chrono::DateTime<chrono::Utc>> = row.get("last_login_at");

    let user_id = UserId::new(&id)
        .map_err(|e| DomainError::storage(format!("Invalid user ID in database: {}", e)))?;
    let team_id = TeamId::new(&team_id)
        .map_err(|e| DomainError::storage(format!("Invalid team ID in database: {}", e)))?;

    let mut user = User::new(
        user_id,
        username,
        password_hash.clone(),
        team_id,
        str_to_role(&team_role),
    );

    // Use internal methods to restore state
    user.set_status(str_to_status(&status));

    // Restore timestamps via serialization workaround
    let mut user_json = serde_json::to_value(&user)
        .map_err(|e| DomainError::storage(format!("Failed to serialize user: {}", e)))?;

    // password_hash is skipped during serialization, so we need to add it back
    user_json["password_hash"] = serde_json::Value::String(password_hash);

    user_json["created_at"] = serde_json::to_value(created_at)
        .map_err(|e| DomainError::storage(format!("Failed to serialize created_at: {}", e)))?;
    user_json["updated_at"] = serde_json::to_value(updated_at)
        .map_err(|e| DomainError::storage(format!("Failed to serialize updated_at: {}", e)))?;

    if let Some(login_at) = last_login_at {
        user_json["last_login_at"] = serde_json::to_value(login_at)
            .map_err(|e| DomainError::storage(format!("Failed to serialize last_login_at: {}", e)))?;
    }

    serde_json::from_value(user_json)
        .map_err(|e| DomainError::storage(format!("Failed to deserialize user: {}", e)))
}

fn status_to_str(status: UserStatus) -> &'static str {
    match status {
        UserStatus::Active => "active",
        UserStatus::Suspended => "suspended",
    }
}

fn str_to_status(s: &str) -> UserStatus {
    match s {
        "suspended" => UserStatus::Suspended,
        _ => UserStatus::Active,
    }
}

fn role_to_str(role: TeamRole) -> &'static str {
    match role {
        TeamRole::Owner => "owner",
        TeamRole::Admin => "admin",
        TeamRole::Member => "member",
    }
}

fn str_to_role(s: &str) -> TeamRole {
    match s {
        "owner" => TeamRole::Owner,
        "admin" => TeamRole::Admin,
        _ => TeamRole::Member,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_conversion() {
        assert_eq!(status_to_str(UserStatus::Active), "active");
        assert_eq!(status_to_str(UserStatus::Suspended), "suspended");

        assert_eq!(str_to_status("active"), UserStatus::Active);
        assert_eq!(str_to_status("suspended"), UserStatus::Suspended);
        assert_eq!(str_to_status("unknown"), UserStatus::Active);
    }

    #[test]
    fn test_role_conversion() {
        assert_eq!(role_to_str(TeamRole::Owner), "owner");
        assert_eq!(role_to_str(TeamRole::Admin), "admin");
        assert_eq!(role_to_str(TeamRole::Member), "member");

        assert_eq!(str_to_role("owner"), TeamRole::Owner);
        assert_eq!(str_to_role("admin"), TeamRole::Admin);
        assert_eq!(str_to_role("member"), TeamRole::Member);
        assert_eq!(str_to_role("unknown"), TeamRole::Member);
    }
}
