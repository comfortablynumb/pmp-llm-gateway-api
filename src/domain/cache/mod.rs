//! Cache domain - Generic caching abstraction layer

mod key;
mod repository;

pub use key::{CacheKey, CacheKeyGenerator, CacheKeyParams, DefaultKeyGenerator};
pub use repository::{Cache, CacheExt};

#[cfg(test)]
pub use repository::mock::MockCache;
