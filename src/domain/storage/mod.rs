//! Storage domain - Generic storage abstraction layer

mod entity;
mod repository;

pub use entity::{StorageEntity, StorageKey};
pub use repository::Storage;

#[cfg(test)]
pub use repository::mock;
