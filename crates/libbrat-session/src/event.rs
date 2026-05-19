//! Events emitted by the session monitor.

use libbrat_grite::SessionStatus;

/// Events emitted by the session monitor for caller observation.
///
/// These events are broadcast to all subscribers and can be used to
/// observe session lifecycle changes in real-time.
#[derive(Debug, Clone)]
pub enum MonitorEvent {
    /// Session spawned successfully.
    Spawned {
        /// Session identifier.
        session_id: String,
        /// Associated task identifier.
        task_id: String,
        /// Process ID.
        pid: u32,
        /// Path to worktree (if polecat session).
        worktree_path: Option<String>,
    },

    /// Session transitioned to Ready state.
    ///
    /// This indicates the engine health check passed and the session
    /// is ready to accept work.
    Ready {
        /// Session identifier.
        session_id: String,
    },

    /// Health check completed.
    HealthCheck {
        /// Session identifier.
        session_id: String,
        /// Whether the session is alive.
        alive: bool,
        /// Number of consecutive health check failures.
        consecutive_failures: u32,
    },

    /// Heartbeat updated in Grite.
    Heartbeat {
        /// Session identifier.
        session_id: String,
    },

    /// Session state changed.
    StateChanged {
        /// Session identifier.
        session_id: String,
        /// Previous status.
        from: SessionStatus,
        /// New status.
        to: SessionStatus,
    },

    /// Session exited.
    Exited {
        /// Session identifier.
        session_id: String,
        /// Exit code.
        exit_code: i32,
        /// Reason for exit.
        exit_reason: String,
    },

    /// Worktree cleaned up.
    WorktreeCleaned {
        /// Session identifier.
        session_id: String,
    },

    /// Non-fatal error occurred.
    Error {
        /// Session identifier (if applicable).
        session_id: Option<String>,
        /// Error description.
        error: String,
    },
}

impl MonitorEvent {
    /// Get the session ID associated with this event, if any.
    pub fn session_id(&self) -> Option<&str> {
        match self {
            MonitorEvent::Spawned { session_id, .. } => Some(session_id),
            MonitorEvent::Ready { session_id } => Some(session_id),
            MonitorEvent::HealthCheck { session_id, .. } => Some(session_id),
            MonitorEvent::Heartbeat { session_id } => Some(session_id),
            MonitorEvent::StateChanged { session_id, .. } => Some(session_id),
            MonitorEvent::Exited { session_id, .. } => Some(session_id),
            MonitorEvent::WorktreeCleaned { session_id } => Some(session_id),
            MonitorEvent::Error { session_id, .. } => session_id.as_deref(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_session_id() {
        let event = MonitorEvent::Ready {
            session_id: "s-20250117-abcd".to_string(),
        };
        assert_eq!(event.session_id(), Some("s-20250117-abcd"));

        let event = MonitorEvent::Error {
            session_id: None,
            error: "test error".to_string(),
        };
        assert_eq!(event.session_id(), None);
    }
}
