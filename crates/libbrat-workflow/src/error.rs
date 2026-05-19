//! Workflow error types.

use thiserror::Error;

/// Errors that can occur during workflow operations.
#[derive(Debug, Error)]
pub enum WorkflowError {
    /// Workflow file not found.
    #[error("workflow not found: {0}")]
    NotFound(String),

    /// Failed to read workflow file.
    #[error("failed to read workflow: {0}")]
    ReadError(#[from] std::io::Error),

    /// Failed to parse YAML.
    #[error("failed to parse workflow YAML: {0}")]
    ParseError(#[from] serde_yaml::Error),

    /// Workflow validation failed.
    #[error("workflow validation failed: {0}")]
    ValidationError(String),

    /// Missing required input.
    #[error("missing required input: {0}")]
    MissingInput(String),

    /// Invalid input value.
    #[error("invalid input '{0}': {1}")]
    InvalidInput(String, String),

    /// Circular dependency in workflow steps.
    #[error("circular dependency detected in workflow steps")]
    CircularDependency,

    /// Unknown step reference in dependency.
    #[error("unknown step '{0}' referenced in 'needs'")]
    UnknownStep(String),

    /// Failed to create convoy/task in Grite.
    #[error("gritee error: {0}")]
    GriteeError(#[from] libbrat_grite::GriteeError),

    /// Workflow directory not found.
    #[error("workflow directory not found: {0}")]
    WorkflowDirNotFound(String),
}
