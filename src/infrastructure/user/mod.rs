//! User infrastructure module
//!
//! This module provides implementations for user authentication and management,
//! including password hashing with Argon2, in-memory repository, and user service.

mod password;
mod postgres_repository;
mod repository;
mod service;

pub use password::{Argon2Hasher, PasswordHasher};
pub use postgres_repository::PostgresUserRepository;
pub use repository::InMemoryUserRepository;
pub use service::{CreateUserRequest, UpdatePasswordRequest, UserService};
