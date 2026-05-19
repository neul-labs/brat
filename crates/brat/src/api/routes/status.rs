//! Status endpoint for a repository.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use libbrat_grite::{SessionStatus, TaskStatus};
use serde::Serialize;

use crate::api::state::DaemonState;

/// Status response for a repository.
#[derive(Serialize)]
pub struct StatusResponse {
    /// Repository ID.
    pub repo_id: String,
    /// Repository path.
    pub repo_path: String,
    /// Number of convoys.
    pub convoys_count: usize,
    /// Number of tasks by status.
    pub tasks_by_status: TasksByStatus,
    /// Number of active sessions.
    pub active_sessions: usize,
}

/// Task counts by status.
#[derive(Serialize, Default)]
pub struct TasksByStatus {
    pub queued: usize,
    pub running: usize,
    pub blocked: usize,
    pub needs_review: usize,
    pub merged: usize,
    pub dropped: usize,
}

/// Error response.
#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

/// GET /api/v1/repos/:repo_id/status
async fn get_status(
    State(state): State<DaemonState>,
    Path(repo_id): Path<String>,
) -> Result<Json<StatusResponse>, (StatusCode, Json<ErrorResponse>)> {
    let ctx = state.get_repo(&repo_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Repository not found: {}", repo_id),
            }),
        )
    })?;

    // Get convoy list
    let convoys = ctx.gritee.convoy_list().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to list convoys: {}", e),
            }),
        )
    })?;

    // Get task list
    let tasks = ctx.gritee.task_list(None).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to list tasks: {}", e),
            }),
        )
    })?;

    // Count tasks by status
    let mut tasks_by_status = TasksByStatus::default();
    for task in &tasks {
        match task.status {
            TaskStatus::Queued => tasks_by_status.queued += 1,
            TaskStatus::Running => tasks_by_status.running += 1,
            TaskStatus::Blocked => tasks_by_status.blocked += 1,
            TaskStatus::NeedsReview => tasks_by_status.needs_review += 1,
            TaskStatus::Merged => tasks_by_status.merged += 1,
            TaskStatus::Dropped => tasks_by_status.dropped += 1,
        }
    }

    // Get session list
    let sessions = ctx.gritee.session_list(None).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to list sessions: {}", e),
            }),
        )
    })?;

    let active_sessions = sessions
        .iter()
        .filter(|s| !matches!(s.status, SessionStatus::Exit))
        .count();

    Ok(Json(StatusResponse {
        repo_id: ctx.id.clone(),
        repo_path: ctx.path.to_string_lossy().to_string(),
        convoys_count: convoys.len(),
        tasks_by_status,
        active_sessions,
    }))
}

/// Build status routes.
pub fn routes() -> Router<DaemonState> {
    Router::new().route("/status", get(get_status))
}
