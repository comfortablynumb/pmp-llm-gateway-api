//! Operation infrastructure implementations

mod in_memory_repository;
mod storage_repository;

pub use in_memory_repository::InMemoryOperationRepository;
pub use storage_repository::StorageOperationRepository;
