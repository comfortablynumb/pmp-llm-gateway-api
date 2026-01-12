//! Team domain module
//!
//! Teams are the primary organizational unit. Every user must belong to a team,
//! and API keys are owned by teams (not individual users).

mod entity;
mod repository;
mod validation;

pub use entity::{Team, TeamId, TeamRole, TeamStatus};
pub use repository::{TeamQuery, TeamRepository};
pub use validation::{validate_team_id, validate_team_name, TeamValidationError};
