//! Knowledge base error types.

/// Errors returned by the knowledge base service.
#[derive(Debug, thiserror::Error)]
pub enum KbError {
    #[error("failed to open zkb tenant: {0}")]
    ZkbOpen(String),

    #[error("zkb operation failed: {0}")]
    ZkbOperation(String),

    #[error("zkb search failed: {0}")]
    ZkbSearch(String),

    #[error("note not found: {0}")]
    NoteNotFound(String),

    #[error("invalid note type: {0}")]
    InvalidNoteType(String),

    #[error("consistency check failed: {0}")]
    ConsistencyCheck(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
