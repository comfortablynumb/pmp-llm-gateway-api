//! Configuration and execution logging domain

mod entity;
mod execution_log;
mod repository;

pub use entity::{
    AppConfiguration, AppConfigurationId, ConfigCategory, ConfigEntry, ConfigKey,
    ConfigValidationError, ConfigValue,
};
pub use execution_log::{
    ExecutionLog, ExecutionLogId, ExecutionLogQuery, ExecutionLogValidationError, ExecutionStats,
    ExecutionStatus, ExecutionType, Executor, TokenUsage, WorkflowStepLog,
};
pub use repository::{ConfigRepository, ExecutionLogRepository};
