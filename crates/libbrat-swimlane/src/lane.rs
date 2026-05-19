//! Swimlane lane definition and management.

use std::collections::VecDeque;
use std::path::PathBuf;

use libbrat_engine::{Engine, SpawnResult};
use serde::{Deserialize, Serialize};

/// Assignment of a task to a swimlane.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAssignment {
    pub task_id: String,
    pub lane_id: String,
    pub component: String,
    pub file_paths: Vec<String>,
}

/// A swimlane representing a parallel agent team.
#[derive(Debug)]
pub struct Swimlane {
    /// Unique lane identifier.
    pub lane_id: String,
    /// Team name (e.g., "backend", "frontend", "tests", "docs").
    pub team: String,
    /// Engine assigned to this lane.
    pub engine_name: String,
    /// Worktree path for this lane.
    pub worktree: PathBuf,
    /// Task queue for this lane.
    pub queue: VecDeque<TaskAssignment>,
    /// Currently active task (if any).
    pub active_task: Option<TaskAssignment>,
    /// Lock namespace for path isolation.
    pub lock_namespace: String,
    /// Maximum number of parallel tasks in this lane (usually 1).
    pub max_parallel: u32,
}

impl Swimlane {
    /// Create a new swimlane.
    pub fn new(
        lane_id: String,
        team: String,
        engine_name: String,
        worktree: PathBuf,
    ) -> Self {
        Self {
            lane_id: lane_id.clone(),
            team,
            engine_name,
            worktree,
            queue: VecDeque::new(),
            active_task: None,
            lock_namespace: lane_id,
            max_parallel: 1,
        }
    }

    /// Whether this lane is currently busy.
    pub fn is_busy(&self) -> bool {
        self.active_task.is_some()
    }

    /// Number of queued tasks.
    pub fn queue_len(&self) -> usize {
        self.queue.len()
    }

    /// Total workload (active + queued).
    pub fn total_workload(&self) -> usize {
        self.active_task.as_ref().map(|_| 1).unwrap_or(0) + self.queue.len()
    }

    /// Whether this lane can accept a task with the given file paths.
    pub fn can_accept_paths(&self, paths: &[PathBuf],
    ) -> bool {
        // Check against active task paths
        if let Some(ref active) = self.active_task {
            for path in paths {
                let path_str = path.to_string_lossy();
                for active_path in &active.file_paths {
                    if path_str.starts_with(active_path) || active_path.starts_with(path_str.as_ref()) {
                        return false;
                    }
                }
            }
        }

        // Check against queued task paths
        for queued in &self.queue {
            for path in paths {
                let path_str = path.to_string_lossy();
                for queued_path in &queued.file_paths {
                    if path_str.starts_with(queued_path) || queued_path.starts_with(path_str.as_ref()) {
                        return false;
                    }
                }
            }
        }

        true
    }
}
