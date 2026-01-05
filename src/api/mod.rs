//! API layer - HTTP endpoints and middleware

pub mod admin;
pub mod health;
pub mod middleware;
pub mod router;
pub mod state;
pub mod types;
pub mod v1;

pub use middleware::RequireApiKey;
pub use router::{create_router, create_router_with_state};
pub use state::AppState;
