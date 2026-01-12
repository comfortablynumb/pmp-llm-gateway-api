//! Infrastructure layer for experiment A/B testing
//!
//! Provides implementations for experiment repositories and utilities.

mod consistent_hashing;
mod in_memory_record_repo;
mod in_memory_repository;
mod statistical;
mod storage_record_repository;
mod storage_repository;

pub use consistent_hashing::ConsistentHasher;
pub use in_memory_record_repo::InMemoryExperimentRecordRepository;
pub use in_memory_repository::InMemoryExperimentRepository;
pub use statistical::{
    calculate_all_significance, calculate_significance, mean, std_dev, variance, welch_t_test,
};
pub use storage_record_repository::StorageExperimentRecordRepository;
pub use storage_repository::StorageExperimentRepository;
