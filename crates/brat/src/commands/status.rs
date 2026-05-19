use std::collections::HashMap;
use std::process::Command;
use std::time::Duration;

use chrono::Utc;
use libbrat_config::InterventionsConfig;
use libbrat_grite::{
    Convoy, GriteeClient, Session, SessionStatus as GriteSessionStatus, Task,
    TaskStatus as GriteTaskStatus,
};
use serde::{Deserialize, Serialize};

use crate::cli::{Cli, StatusArgs};
use crate::context::BratContext;
use crate::error::BratError;
use crate::output::{clear_screen, output_stream, output_success};

/// Output of the status command.
///
/// This matches the schema in docs/brat-status-schema.md.
#[derive(Debug, Serialize)]
pub struct StatusOutput {
    /// Schema version for compatibility.
    pub schema_version: u32,

    /// Timestamp when this status was generated.
    pub generated_ts: i64,

    /// Path to the repository root.
    pub repo_root: String,

    /// Active convoys.
    pub convoys: Vec<ConvoyStatus>,

    /// Task summary.
    pub tasks: TaskSummary,

    /// Active sessions.
    pub sessions: Vec<SessionStatus>,

    /// Merge queue status.
    pub merge_queue: MergeQueueStatus,

    /// Active locks.
    pub locks: Vec<LockStatus>,

    /// Required interventions.
    pub interventions: Vec<Intervention>,
}

/// Convoy status.
#[derive(Debug, Serialize)]
pub struct ConvoyStatus {
    pub convoy_id: String,
    pub title: String,
    pub status: String,
    pub task_counts: TaskCounts,
}

/// Task counts by status.
#[derive(Debug, Serialize)]
pub struct TaskCounts {
    pub queued: u32,
    pub running: u32,
    pub blocked: u32,
    pub needs_review: u32,
    pub merged: u32,
    pub dropped: u32,
}

impl Default for TaskCounts {
    fn default() -> Self {
        Self {
            queued: 0,
            running: 0,
            blocked: 0,
            needs_review: 0,
            merged: 0,
            dropped: 0,
        }
    }
}

/// Task summary across all convoys.
#[derive(Debug, Serialize)]
pub struct TaskSummary {
    pub total: u32,
    pub by_status: TaskCounts,
}

impl Default for TaskSummary {
    fn default() -> Self {
        Self {
            total: 0,
            by_status: TaskCounts::default(),
        }
    }
}

/// Session status.
#[derive(Debug, Serialize)]
pub struct SessionStatus {
    pub session_id: String,
    pub task_id: String,
    pub role: String,
    pub session_type: String,
    pub engine: String,
    pub state: String,
    pub last_heartbeat_ts: Option<i64>,
}

/// Merge queue status.
#[derive(Debug, Serialize)]
pub struct MergeQueueStatus {
    pub queued: u32,
    pub running: u32,
    pub failed: u32,
    pub succeeded: u32,
}

impl Default for MergeQueueStatus {
    fn default() -> Self {
        Self {
            queued: 0,
            running: 0,
            failed: 0,
            succeeded: 0,
        }
    }
}

/// Lock status.
#[derive(Debug, Serialize)]
pub struct LockStatus {
    pub resource: String,
    pub owner: String,
    pub expires_ts: i64,
}

/// Intervention required.
#[derive(Debug, Serialize)]
pub struct Intervention {
    pub kind: String,
    pub summary: String,
    pub task_id: Option<String>,
    pub session_id: Option<String>,
    pub cognitive_prompt: String,
    pub recommended_actions: Vec<String>,
}

/// Run the status command.
pub fn run(cli: &Cli, args: &StatusArgs) -> Result<(), BratError> {
    if args.watch {
        run_watch(cli, args)
    } else {
        let output = build_status(cli, args)?;
        output_success(cli, output);
        Ok(())
    }
}

/// Run status in watch mode (polling loop).
fn run_watch(cli: &Cli, args: &StatusArgs) -> Result<(), BratError> {
    let poll_interval = Duration::from_secs(args.poll_interval);

    loop {
        // Clear screen for human output (no-op in JSON mode)
        clear_screen(cli);

        // Build and output status
        let output = build_status(cli, args)?;
        output_stream(cli, output);

        // Sleep until next poll
        std::thread::sleep(poll_interval);
    }
}

/// Build the status output (shared by run_once and run_watch).
fn build_status(cli: &Cli, args: &StatusArgs) -> Result<StatusOutput, BratError> {
    let ctx = BratContext::resolve(cli)?;

    // Require that brat is initialized
    let _config = ctx.require_initialized()?;

    // Check if gritee is initialized
    if !ctx.is_gritee_initialized() {
        // Return empty status if gritee not initialized
        return Ok(StatusOutput {
            schema_version: 1,
            generated_ts: Utc::now().timestamp_millis(),
            repo_root: ctx.repo_root.display().to_string(),
            convoys: Vec::new(),
            tasks: TaskSummary::default(),
            sessions: Vec::new(),
            merge_queue: MergeQueueStatus::default(),
            locks: Vec::new(),
            interventions: Vec::new(),
        });
    }

    let client = ctx.gritee_client();

    // Query convoys from Gritee
    let convoys = client.convoy_list().unwrap_or_default();

    // Query tasks from Gritee
    let all_tasks = client.task_list(None).unwrap_or_default();

    // Filter by convoy if specified
    let (convoys, tasks): (Vec<Convoy>, Vec<Task>) = if let Some(ref convoy_id) = args.convoy {
        let filtered_convoys: Vec<Convoy> = convoys
            .into_iter()
            .filter(|c| c.convoy_id == *convoy_id)
            .collect();
        let filtered_tasks: Vec<Task> = all_tasks
            .into_iter()
            .filter(|t| t.convoy_id == *convoy_id)
            .collect();
        (filtered_convoys, filtered_tasks)
    } else {
        (convoys, all_tasks)
    };

    // Build task counts per convoy
    let mut convoy_task_counts: HashMap<String, TaskCounts> = HashMap::new();
    for task in &tasks {
        let counts = convoy_task_counts
            .entry(task.convoy_id.clone())
            .or_default();
        match task.status {
            GriteTaskStatus::Queued => counts.queued += 1,
            GriteTaskStatus::Running => counts.running += 1,
            GriteTaskStatus::Blocked => counts.blocked += 1,
            GriteTaskStatus::NeedsReview => counts.needs_review += 1,
            GriteTaskStatus::Merged => counts.merged += 1,
            GriteTaskStatus::Dropped => counts.dropped += 1,
        }
    }

    // Build convoy status list
    let convoy_statuses: Vec<ConvoyStatus> = convoys
        .into_iter()
        .map(|c| {
            let task_counts = convoy_task_counts
                .remove(&c.convoy_id)
                .unwrap_or_default();
            ConvoyStatus {
                convoy_id: c.convoy_id,
                title: c.title,
                status: format!("{:?}", c.status).to_lowercase(),
                task_counts,
            }
        })
        .collect();

    // Compute total task summary
    let total_tasks = tasks.len() as u32;
    let mut total_counts = TaskCounts::default();
    for task in &tasks {
        match task.status {
            GriteTaskStatus::Queued => total_counts.queued += 1,
            GriteTaskStatus::Running => total_counts.running += 1,
            GriteTaskStatus::Blocked => total_counts.blocked += 1,
            GriteTaskStatus::NeedsReview => total_counts.needs_review += 1,
            GriteTaskStatus::Merged => total_counts.merged += 1,
            GriteTaskStatus::Dropped => total_counts.dropped += 1,
        }
    }

    // Query sessions from Gritee
    let gritee_sessions: Vec<Session> = if let Some(ref convoy_id) = args.convoy {
        // Get sessions for tasks in this convoy only
        let mut sessions = Vec::new();
        for task in &tasks {
            if task.convoy_id == *convoy_id {
                if let Ok(task_sessions) = client.session_list(Some(&task.task_id)) {
                    sessions.extend(task_sessions);
                }
            }
        }
        sessions
    } else {
        client.session_list(None).unwrap_or_default()
    };

    // Convert to output format, filtering out Exit sessions
    let session_statuses: Vec<SessionStatus> = gritee_sessions
        .into_iter()
        .filter(|s| s.status != GriteSessionStatus::Exit)
        .map(|s| SessionStatus {
            session_id: s.session_id,
            task_id: s.task_id,
            role: s.role.as_str().to_string(),
            session_type: s.session_type.as_str().to_string(),
            engine: s.engine,
            state: format!("{}", s.status),
            last_heartbeat_ts: s.last_heartbeat_ts,
        })
        .collect();

    // Get config for intervention thresholds
    let interventions_config = ctx
        .config
        .as_ref()
        .map(|c| c.interventions.clone())
        .unwrap_or_default();

    // Query merge queue status
    let merge_queue = query_merge_queue(&client);

    // Query locks
    let locks = query_locks(&ctx.repo_root);

    // Detect interventions
    let now_ms = Utc::now().timestamp_millis();
    let interventions = detect_interventions(
        &session_statuses,
        &tasks,
        &interventions_config,
        now_ms,
    );

    // Handle --all-repos flag
    if args.all_repos {
        // TODO: Aggregate across all repos in config.repos.roots
        // For now, just return single repo status
    }

    Ok(StatusOutput {
        schema_version: 1,
        generated_ts: now_ms,
        repo_root: ctx.repo_root.display().to_string(),
        convoys: convoy_statuses,
        tasks: TaskSummary {
            total: total_tasks,
            by_status: total_counts,
        },
        sessions: session_statuses,
        merge_queue,
        locks,
        interventions,
    })
}

// =============================================================================
// Helper functions for status queries
// =============================================================================

/// Query merge queue status from Gritee.
///
/// Counts tasks with merge:* labels.
fn query_merge_queue(client: &GriteeClient) -> MergeQueueStatus {
    let mut status = MergeQueueStatus::default();

    // Query all tasks and check for merge labels
    // Note: In a full implementation, we'd use issue_list with label filters
    // For now, we'll query tasks and check their labels through the task status
    if let Ok(tasks) = client.task_list(None) {
        for task in tasks {
            // Tasks in NeedsReview status are candidates for merge queue
            if task.status == GriteTaskStatus::NeedsReview {
                status.queued += 1;
            }
        }
    }

    // Note: merge:running, merge:failed, merge:succeeded tracking
    // would require label queries which aren't directly supported yet
    // This is a simplified implementation

    status
}

/// Grite lock status JSON response envelope.
#[derive(Debug, Deserialize)]
struct GriteLockResponse {
    #[allow(dead_code)]
    ok: bool,
    data: Option<GriteLockData>,
    #[allow(dead_code)]
    error: Option<GriteLockError>,
}

#[derive(Debug, Deserialize)]
struct GriteLockData {
    #[serde(default)]
    locks: Vec<GriteLockEntry>,
}

#[derive(Debug, Deserialize)]
struct GriteLockEntry {
    resource: String,
    owner: String,
    #[serde(default)]
    expires_ts: i64,
}

#[derive(Debug, Deserialize)]
struct GriteLockError {
    #[allow(dead_code)]
    message: String,
}

/// Query locks from Gritee.
///
/// Shells out to `gritee lock status --json`.
fn query_locks(repo_root: &std::path::Path) -> Vec<LockStatus> {
    let output = Command::new("gritee")
        .args(["lock", "status", "--json"])
        .current_dir(repo_root)
        .output();

    match output {
        Ok(result) if result.status.success() => {
            let stdout = String::from_utf8_lossy(&result.stdout);
            if let Ok(response) = serde_json::from_str::<GriteLockResponse>(&stdout) {
                if let Some(data) = response.data {
                    return data
                        .locks
                        .into_iter()
                        .map(|entry| LockStatus {
                            resource: entry.resource,
                            owner: entry.owner,
                            expires_ts: entry.expires_ts,
                        })
                        .collect();
                }
            }
            Vec::new()
        }
        _ => Vec::new(), // Graceful fallback if gritee lock status isn't available
    }
}

/// Detect interventions based on current state.
fn detect_interventions(
    sessions: &[SessionStatus],
    tasks: &[Task],
    config: &InterventionsConfig,
    now_ms: i64,
) -> Vec<Intervention> {
    let mut interventions = Vec::new();

    // Detect stale sessions (no heartbeat for stale_session_ms)
    for session in sessions {
        if let Some(heartbeat_ts) = session.last_heartbeat_ts {
            let age_ms = now_ms - heartbeat_ts;
            if age_ms > config.stale_session_ms as i64 {
                interventions.push(Intervention {
                    kind: "stuck_session".to_string(),
                    summary: format!(
                        "Session {} missed heartbeat for {}m",
                        session.session_id,
                        age_ms / 60_000
                    ),
                    task_id: Some(session.task_id.clone()),
                    session_id: Some(session.session_id.clone()),
                    cognitive_prompt:
                        "Decide whether to wait, restart the session, or reassign the task."
                            .to_string(),
                    recommended_actions: vec![
                        format!("brat session show {}", session.session_id),
                        format!("brat session stop {}", session.session_id),
                        "brat witness run --once".to_string(),
                    ],
                });
            }
        }
    }

    // Detect blocked tasks
    for task in tasks {
        if task.status == GriteTaskStatus::Blocked {
            interventions.push(Intervention {
                kind: "blocked_task".to_string(),
                summary: format!("Task {} is blocked", task.task_id),
                task_id: Some(task.task_id.clone()),
                session_id: None,
                cognitive_prompt:
                    "Determine what information is missing and add it to the task.".to_string(),
                recommended_actions: vec![
                    format!("brat task update {} --status running", task.task_id),
                ],
            });
        }
    }

    // Sort by severity (stuck_session > blocked_task > others)
    interventions.sort_by_key(|i| match i.kind.as_str() {
        "stuck_session" => 0,
        "blocked_task" => 1,
        "merge_failed" => 2,
        _ => 3,
    });

    interventions
}
