//! Handle for controlling a monitored session.

use libbrat_engine::StopMode;
use libbrat_grite::SessionStatus;
use tokio::sync::{mpsc, oneshot};

use crate::error::SessionMonitorError;

/// Internal commands sent to the monitoring task.
#[derive(Debug)]
pub(crate) enum MonitorCommand {
    /// Request a state transition.
    Transition {
        new_status: SessionStatus,
        reply: oneshot::Sender<Result<(), SessionMonitorError>>,
    },
    /// Stop the session.
    Stop {
        mode: StopMode,
        reply: oneshot::Sender<Result<(), SessionMonitorError>>,
    },
    /// Shutdown the monitoring task.
    Shutdown,
}

/// Handle to control a monitored session.
///
/// This handle is returned by `SessionMonitor::spawn_session` and provides
/// methods to control the session's lifecycle.
#[derive(Debug, Clone)]
pub struct MonitorHandle {
    session_id: String,
    command_tx: mpsc::Sender<MonitorCommand>,
}

impl MonitorHandle {
    /// Create a new monitor handle.
    pub(crate) fn new(session_id: String, command_tx: mpsc::Sender<MonitorCommand>) -> Self {
        Self {
            session_id,
            command_tx,
        }
    }

    /// Get the session ID.
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Request a state transition.
    ///
    /// The transition will be validated against the state machine before
    /// being applied.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The transition is invalid according to the state machine
    /// - The monitor task has been shut down
    /// - Communication with the monitor task fails
    pub async fn transition(
        &self,
        new_status: SessionStatus,
    ) -> Result<(), SessionMonitorError> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.command_tx
            .send(MonitorCommand::Transition {
                new_status,
                reply: reply_tx,
            })
            .await
            .map_err(|_| SessionMonitorError::Shutdown)?;

        reply_rx
            .await
            .map_err(|_| SessionMonitorError::ChannelError("reply channel closed".to_string()))?
    }

    /// Stop the session.
    ///
    /// This sends a stop command to the engine. The session will be marked
    /// as exited once the process terminates.
    ///
    /// # Arguments
    ///
    /// * `mode` - How to stop the session (graceful or kill).
    pub async fn stop(&self, mode: StopMode) -> Result<(), SessionMonitorError> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.command_tx
            .send(MonitorCommand::Stop {
                mode,
                reply: reply_tx,
            })
            .await
            .map_err(|_| SessionMonitorError::Shutdown)?;

        reply_rx
            .await
            .map_err(|_| SessionMonitorError::ChannelError("reply channel closed".to_string()))?
    }

    /// Request shutdown of the monitoring task.
    ///
    /// This is typically called by SessionMonitor during shutdown.
    pub(crate) async fn shutdown(&self) -> Result<(), SessionMonitorError> {
        self.command_tx
            .send(MonitorCommand::Shutdown)
            .await
            .map_err(|_| SessionMonitorError::Shutdown)
    }
}
