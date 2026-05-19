//! Workflow-specific error types.

use libbrat_engine::EngineError;
use libbrat_grite::GriteeError;
use libbrat_session::SessionMonitorError;
use libbrat_worktree::WorktreeError;
use thiserror::Error;

/// Errors that can occur during workflow execution.
#[derive(Debug, Error)]
pub enum WorkflowError {
    /// Grite operation failed.
    #[error("gritee error: {0}")]
    Grite(#[from] GriteeError),

    /// Session monitor operation failed.
    #[error("session error: {0}")]
    Session(#[from] SessionMonitorError),

    /// Engine operation failed.
    #[error("engine error: {0}")]
    Engine(#[from] EngineError),

    /// Worktree operation failed.
    #[error("worktree error: {0}")]
    Worktree(#[from] WorktreeError),

    /// Git command failed.
    #[error("git command failed: {0}")]
    GitFailed(String),

    /// IO error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Invalid task.
    #[error("invalid task: {0}")]
    InvalidTask(String),

    /// Role is disabled in configuration.
    #[error("role disabled: {0}")]
    RoleDisabled(String),

    /// Lock acquisition failed due to conflict.
    #[error("lock conflict on {resource}: held by {holder:?}")]
    LockConflict {
        resource: String,
        holder: Option<String>,
    },

    /// Lock operation failed.
    #[error("lock operation failed: {0}")]
    LockFailed(String),
}
