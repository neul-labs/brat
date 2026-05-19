//! Session reconciliation workflow for crash recovery.
//!
//! This module provides the `ReconcileWorkflow` for detecting and recovering
//! from crashed sessions and cleaning up orphaned worktrees.

use std::collections::HashSet;

use libbrat_config::InterventionsConfig;
use libbrat_grite::{GriteeClient, SessionStatus};
use libbrat_worktree::WorktreeManager;
use serde::Serialize;

use super::WorkflowError;

/// Result of a reconciliation run.
#[derive(Debug, Default, Serialize)]
pub struct ReconcileResult {
    /// Number of sessions checked.
    pub sessions_checked: usize,

    /// Number of sessions marked as crashed.
    pub sessions_marked_crashed: usize,

    /// Session IDs that were marked as crashed.
    pub crashed_session_ids: Vec<String>,

    /// Number of worktrees cleaned up.
    pub worktrees_cleaned: usize,

    /// Worktree session IDs that were cleaned.
    pub cleaned_worktree_ids: Vec<String>,

    /// Errors encountered during reconciliation.
    pub errors: Vec<String>,
}

impl ReconcileResult {
    /// Returns true if any reconciliation actions were taken.
    pub fn had_actions(&self) -> bool {
        self.sessions_marked_crashed > 0 || self.worktrees_cleaned > 0
    }
}

/// Workflow for session reconciliation.
///
/// Reconciles Grite session state with reality by:
/// 1. Detecting stale sessions (no heartbeat for stale_session_ms)
/// 2. Marking crashed sessions as Exit
/// 3. Cleaning up orphaned worktrees
pub struct ReconcileWorkflow {
    gritee: GriteeClient,
    worktree_manager: Option<WorktreeManager>,
    stale_session_ms: u64,
}

impl ReconcileWorkflow {
    /// Create a new ReconcileWorkflow.
    ///
    /// # Arguments
    ///
    /// * `gritee` - Gritee client for session state.
    /// * `worktree_manager` - Optional worktree manager for cleanup.
    /// * `config` - Interventions config with stale session threshold.
    pub fn new(
        gritee: GriteeClient,
        worktree_manager: Option<WorktreeManager>,
        config: InterventionsConfig,
    ) -> Self {
        Self {
            gritee,
            worktree_manager,
            stale_session_ms: config.stale_session_ms,
        }
    }

    /// Run reconciliation once.
    ///
    /// This performs the following steps:
    /// 1. Get all active sessions from Gritee
    /// 2. Identify stale sessions based on heartbeat
    /// 3. Mark stale sessions as crashed (Exit status)
    /// 4. Clean up orphaned worktrees
    pub fn run_once(&self) -> Result<ReconcileResult, WorkflowError> {
        let mut result = ReconcileResult::default();

        // Step 1: Get active sessions
        let sessions = self.gritee.session_list(None)?;
        let active_sessions: Vec<_> = sessions
            .into_iter()
            .filter(|s| s.status != SessionStatus::Exit)
            .collect();

        result.sessions_checked = active_sessions.len();

        // Step 2: Check for stale sessions
        let now_ms = current_time_ms();
        let mut active_session_ids: HashSet<String> = HashSet::new();

        for session in &active_sessions {
            let age_ms = match session.last_heartbeat_ts {
                Some(ts) => now_ms - ts,
                None => now_ms - session.started_ts,
            };

            if age_ms > self.stale_session_ms as i64 && age_ms > 0 {
                // Session is stale - mark as crashed
                match self.gritee.session_exit(
                    &session.session_id,
                    -1,
                    "crash-recovery",
                    None,
                ) {
                    Ok(()) => {
                        result.sessions_marked_crashed += 1;
                        result.crashed_session_ids.push(session.session_id.clone());
                    }
                    Err(e) => {
                        result.errors.push(format!(
                            "Failed to mark session {} as crashed: {}",
                            session.session_id, e
                        ));
                    }
                }
            } else {
                // Session is still active
                active_session_ids.insert(session.session_id.clone());
            }
        }

        // Step 3: Clean up orphaned worktrees
        if let Some(ref wm) = self.worktree_manager {
            match wm.cleanup_stale(&active_session_ids) {
                Ok(report) => {
                    result.worktrees_cleaned = report.cleaned.len();
                    result.cleaned_worktree_ids = report.cleaned;
                    for (session_id, err) in report.errors {
                        result.errors.push(format!(
                            "Failed to clean worktree {}: {}",
                            session_id, err
                        ));
                    }
                }
                Err(e) => {
                    result.errors.push(format!("Worktree cleanup failed: {}", e));
                }
            }
        }

        Ok(result)
    }
}

/// Get current time in milliseconds.
fn current_time_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reconcile_result_had_actions() {
        let empty = ReconcileResult::default();
        assert!(!empty.had_actions());

        let with_crashed = ReconcileResult {
            sessions_marked_crashed: 1,
            ..Default::default()
        };
        assert!(with_crashed.had_actions());

        let with_cleaned = ReconcileResult {
            worktrees_cleaned: 1,
            ..Default::default()
        };
        assert!(with_cleaned.had_actions());
    }
}
