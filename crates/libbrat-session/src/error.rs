//! Error types for session monitor operations.

use libbrat_engine::EngineError;
use libbrat_grite::GriteeError;
use libbrat_worktree::WorktreeError;
use thiserror::Error;

/// Errors that can occur during session monitoring.
#[derive(Debug, Error)]
pub enum SessionMonitorError {
    /// Engine operation failed.
    #[error("engine error: {0}")]
    Engine(#[from] EngineError),

    /// Grite operation failed.
    #[error("gritee error: {0}")]
    Grite(#[from] GriteeError),

    /// Worktree operation failed.
    #[error("worktree error: {0}")]
    Worktree(#[from] WorktreeError),

    /// Session not found in monitor.
    #[error("session not found: {0}")]
    SessionNotFound(String),

    /// Session already being monitored.
    #[error("session already monitored: {0}")]
    AlreadyMonitored(String),

    /// Spawn coordination failed.
    #[error("spawn failed: {0}")]
    SpawnFailed(String),

    /// Invalid state transition requested.
    #[error("invalid state transition: {0}")]
    InvalidTransition(String),

    /// Monitor has been shut down.
    #[error("monitor shutdown")]
    Shutdown,

    /// Channel communication error.
    #[error("channel error: {0}")]
    ChannelError(String),
}
