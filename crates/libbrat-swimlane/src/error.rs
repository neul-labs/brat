//! Swimlane error types.

/// Errors returned by the swimlane scheduler.
#[derive(Debug, thiserror::Error)]
pub enum SwimlaneError {
    #[error("no available swimlane for task")]
    NoAvailableLane,

    #[error("swimlane '{0}' not found")]
    LaneNotFound(String),

    #[error("task '{0}' already assigned to lane '{1}'")]
    AlreadyAssigned(String, String),

    #[error("path conflict: '{0}' is locked by lane '{1}'")]
    PathConflict(String, String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
