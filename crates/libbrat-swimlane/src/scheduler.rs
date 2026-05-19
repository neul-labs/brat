//! Swimlane scheduler for assigning architecture components to lanes.

use std::collections::HashMap;
use std::path::PathBuf;

use libbrat_config::SwimlanesConfig;
use tracing::{debug, info, warn};

use crate::error::SwimlaneError;
use crate::lane::{Swimlane, TaskAssignment};

/// Result of scheduling a task.
#[derive(Debug, Clone)]
pub struct ScheduleResult {
    pub lane_id: String,
    pub task_id: String,
}

/// Swimlane scheduler that assigns tasks to parallel agent teams.
#[derive(Debug)]
pub struct SwimlaneScheduler {
    lanes: Vec<Swimlane>,
}

impl SwimlaneScheduler {
    /// Create a scheduler from configuration.
    pub fn from_config(
        config: &SwimlanesConfig,
        repo_root: &std::path::Path,
    ) -> Result<Self, SwimlaneError> {
        let mut lanes = Vec::new();
        let worktree_root = repo_root.join(".brat").join("swimlanes");

        for (i, team) in config.teams.iter().enumerate() {
            let engine = config
                .engines
                .get(team)
                .cloned()
                .unwrap_or_else(|| "codex".to_string());

            let lane_id = format!("{}-{}", team, i);
            let worktree = worktree_root.join(&lane_id);

            lanes.push(Swimlane::new(
                lane_id,
                team.clone(),
                engine,
                worktree,
            ));
        }

        info!(lane_count = lanes.len(), "swimlane scheduler created");
        Ok(Self { lanes })
    }

    /// Assign a task to the best available swimlane.
    pub fn assign_task(
        &mut self,
        task_id: String,
        component: String,
        file_paths: Vec<String>,
    ) -> Result<ScheduleResult, SwimlaneError> {
        let paths: Vec<PathBuf> = file_paths.iter().map(|p| PathBuf::from(p)).collect();

        // Find the best lane: least busy, can accept paths
        let best = self
            .lanes
            .iter_mut()
            .filter(|lane| lane.can_accept_paths(&paths))
            .min_by_key(|lane| lane.total_workload());

        if let Some(lane) = best {
            let assignment = TaskAssignment {
                task_id: task_id.clone(),
                lane_id: lane.lane_id.clone(),
                component: component.clone(),
                file_paths: file_paths.clone(),
            };

            lane.queue.push_back(assignment);

            debug!(
                task_id = %task_id,
                lane_id = %lane.lane_id,
                component = %component,
                "task assigned to swimlane"
            );

            Ok(ScheduleResult {
                lane_id: lane.lane_id.clone(),
                task_id,
            })
        } else {
            warn!(
                task_id = %task_id,
                component = %component,
                "no available swimlane for task"
            );
            Err(SwimlaneError::NoAvailableLane)
        }
    }

    /// Release a lane after task completion.
    pub fn release_lane(
        &mut self,
        lane_id: &str,
    ) -> Result<(), SwimlaneError> {
        if let Some(lane) = self.lanes.iter_mut().find(|l| l.lane_id == lane_id) {
            lane.active_task = None;

            // Promote next queued task to active
            if let Some(next) = lane.queue.pop_front() {
                lane.active_task = Some(next);
            }

            info!(lane_id = %lane_id, "lane released");
            Ok(())
        } else {
            Err(SwimlaneError::LaneNotFound(lane_id.to_string()))
        }
    }

    /// Get all lanes.
    pub fn lanes(&self) -> &[Swimlane] {
        &self.lanes
    }

    /// Get a mutable reference to a lane.
    pub fn lane_mut(&mut self, lane_id: &str) -> Option<&mut Swimlane> {
        self.lanes.iter_mut().find(|l| l.lane_id == lane_id)
    }

    /// Total workload across all lanes.
    pub fn total_workload(&self) -> usize {
        self.lanes.iter().map(|l| l.total_workload()).sum()
    }

    /// Number of active tasks across all lanes.
    pub fn active_count(&self) -> usize {
        self.lanes.iter().filter(|l| l.is_busy()).count()
    }

    /// Number of queued tasks across all lanes.
    pub fn queued_count(&self) -> usize {
        self.lanes.iter().map(|l| l.queue_len()).sum()
    }
}
