use std::io;

use libbrat_config::ConfigError;
use libbrat_grite::GriteeError;

use crate::workflows::WorkflowError;

/// Brat CLI errors.
#[derive(Debug, thiserror::Error)]
pub enum BratError {
    /// Not in a git repository.
    #[error("not a git repository (or any parent up to mount point)")]
    NotAGitRepo,

    /// Brat is not initialized in this repository.
    #[error("brat not initialized in this repository (run 'brat init' first)")]
    NotInitialized,

    /// Grite is not initialized in this repository.
    #[error("gritee not initialized in this repository (run 'brat init' first)")]
    GriteeNotInitialized,

    /// Brat is already initialized.
    #[error("brat already initialized in this repository")]
    AlreadyInitialized,

    /// Grite initialization failed.
    #[error("gritee init failed: {0}")]
    GriteeInitFailed(String),

    /// Grite command failed.
    #[error("gritee command failed: {0}")]
    GriteeCommandFailed(String),

    /// Grite error.
    #[error("gritee error: {0}")]
    Gritee(#[from] GriteeError),

    /// Configuration error.
    #[error("config error: {0}")]
    Config(#[from] ConfigError),

    /// IO error.
    #[error("io error: {0}")]
    Io(#[from] io::Error),

    /// JSON serialization error.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// Role is disabled in configuration.
    #[error("role disabled: {0}")]
    RoleDisabled(String),

    /// Workflow error.
    #[error("workflow error: {0}")]
    Workflow(#[from] WorkflowError),

    /// Other error.
    #[error("{0}")]
    Other(String),
}

impl BratError {
    /// Returns an error code for JSON output.
    pub fn error_code(&self) -> &'static str {
        match self {
            BratError::NotAGitRepo => "not_git_repo",
            BratError::NotInitialized => "not_initialized",
            BratError::GriteeNotInitialized => "gritee_not_initialized",
            BratError::AlreadyInitialized => "already_initialized",
            BratError::GriteeInitFailed(_) => "gritee_init_failed",
            BratError::GriteeCommandFailed(_) => "gritee_command_failed",
            BratError::Gritee(_) => "gritee_error",
            BratError::Config(_) => "config_error",
            BratError::Io(_) => "io_error",
            BratError::Json(_) => "json_error",
            BratError::RoleDisabled(_) => "role_disabled",
            BratError::Workflow(_) => "workflow_error",
            BratError::Other(_) => "other_error",
        }
    }

    /// Returns an exit code for the CLI.
    pub fn exit_code(&self) -> i32 {
        match self {
            BratError::NotAGitRepo => 2,
            BratError::NotInitialized => 3,
            BratError::GriteeNotInitialized => 4,
            BratError::AlreadyInitialized => 5,
            BratError::GriteeInitFailed(_) => 6,
            BratError::GriteeCommandFailed(_) => 7,
            BratError::Gritee(_) => 8,
            BratError::Config(_) => 9,
            BratError::Io(_) => 10,
            BratError::Json(_) => 11,
            BratError::RoleDisabled(_) => 12,
            BratError::Workflow(_) => 13,
            BratError::Other(_) => 1,
        }
    }
}
