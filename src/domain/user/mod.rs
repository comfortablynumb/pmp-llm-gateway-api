//! User domain
//!
//! This module provides domain types and traits for user authentication,
//! including user entities, validation, and repository traits.

mod entity;
mod repository;
mod validation;

pub use entity::{User, UserId, UserStatus};
pub use repository::UserRepository;
pub use validation::{
    validate_password, validate_user_id, validate_username, UserValidationError,
};

#[cfg(test)]
pub use repository::mock::MockUserRepository;
