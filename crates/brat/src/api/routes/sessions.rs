//! Session management endpoints.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use libbrat_grite::SessionStatus;
use serde::{Deserialize, Serialize};
use std::process::Command;

use crate::api::state::DaemonState;

use super::status::ErrorResponse;

/// Session response.
#[derive(Serialize)]
pub struct SessionResponse {
    pub session_id: String,
    pub task_id: String,
    pub gritee_issue_id: String,
    pub engine: String,
    pub status: String,
    pub pid: Option<u32>,
    pub worktree: Option<String>,
    pub started_ts: i64,
    pub exit_code: Option<i32>,
    pub exit_reason: Option<String>,
}

/// Query parameters for listing sessions.
#[derive(Deserialize, Default)]
pub struct ListSessionsQuery {
    /// Filter by task ID.
    pub task: Option<String>,
}

/// Request to stop a session.
#[derive(Deserialize)]
pub struct StopSessionRequest {
    #[serde(default = "default_stop_reason")]
    pub reason: String,
}

fn default_stop_reason() -> String {
    "api-stop".to_string()
}

fn session_status_to_string(status: SessionStatus) -> String {
    match status {
        SessionStatus::Spawned => "spawned".to_string(),
        SessionStatus::Ready => "ready".to_string(),
        SessionStatus::Running => "running".to_string(),
        SessionStatus::Handoff => "handoff".to_string(),
        SessionStatus::Exit => "exit".to_string(),
    }
}

/// GET /api/v1/repos/:repo_id/sessions
async fn list_sessions(
    State(state): State<DaemonState>,
    Path(repo_id): Path<String>,
    Query(query): Query<ListSessionsQuery>,
) -> Result<Json<Vec<SessionResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let ctx = state.get_repo(&repo_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Repository not found: {}", repo_id),
            }),
        )
    })?;

    let sessions = ctx
        .gritee
        .session_list(query.task.as_deref())
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to list sessions: {}", e),
                }),
            )
        })?;

    let responses: Vec<SessionResponse> = sessions
        .into_iter()
        .map(|s| SessionResponse {
            session_id: s.session_id,
            task_id: s.task_id,
            gritee_issue_id: s.gritee_issue_id,
            engine: s.engine,
            status: session_status_to_string(s.status),
            pid: s.pid,
            worktree: if s.worktree.is_empty() {
                None
            } else {
                Some(s.worktree)
            },
            started_ts: s.started_ts,
            exit_code: s.exit_code,
            exit_reason: s.exit_reason,
        })
        .collect();

    Ok(Json(responses))
}

/// GET /api/v1/repos/:repo_id/sessions/:session_id
async fn get_session(
    State(state): State<DaemonState>,
    Path((repo_id, session_id)): Path<(String, String)>,
) -> Result<Json<SessionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let ctx = state.get_repo(&repo_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Repository not found: {}", repo_id),
            }),
        )
    })?;

    // List sessions and find the one with matching ID
    let sessions = ctx.gritee.session_list(None).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to list sessions: {}", e),
            }),
        )
    })?;

    let session = sessions
        .into_iter()
        .find(|s| s.session_id == session_id)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Session not found: {}", session_id),
                }),
            )
        })?;

    Ok(Json(SessionResponse {
        session_id: session.session_id,
        task_id: session.task_id,
        gritee_issue_id: session.gritee_issue_id,
        engine: session.engine,
        status: session_status_to_string(session.status),
        pid: session.pid,
        worktree: if session.worktree.is_empty() {
            None
        } else {
            Some(session.worktree)
        },
        started_ts: session.started_ts,
        exit_code: session.exit_code,
        exit_reason: session.exit_reason,
    }))
}

/// POST /api/v1/repos/:repo_id/sessions/:session_id/stop
async fn stop_session(
    State(state): State<DaemonState>,
    Path((repo_id, session_id)): Path<(String, String)>,
    Json(req): Json<StopSessionRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let ctx = state.get_repo(&repo_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Repository not found: {}", repo_id),
            }),
        )
    })?;

    // Use session_exit with exit code -1 to indicate user stop
    ctx.gritee
        .session_exit(&session_id, -1, &req.reason, None)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to stop session: {}", e),
                }),
            )
        })?;

    Ok(StatusCode::NO_CONTENT)
}

/// Query parameters for getting session logs.
#[derive(Deserialize, Default)]
pub struct SessionLogsQuery {
    /// Number of lines to return (default: 100).
    #[serde(default = "default_log_lines")]
    pub lines: usize,
}

fn default_log_lines() -> usize {
    100
}

/// Response with session logs.
#[derive(Serialize)]
pub struct SessionLogsResponse {
    /// Log lines.
    pub lines: Vec<String>,
    /// Whether there are more lines available.
    pub has_more: bool,
}

/// Read blob content from git.
fn read_blob(repo_root: &std::path::Path, blob_ref: &str) -> Result<String, String> {
    // The blob ref might be in format "sha256:xxxx" or just a git hash
    let hash = if let Some(stripped) = blob_ref.strip_prefix("sha256:") {
        stripped
    } else {
        blob_ref
    };

    let output = Command::new("git")
        .args(["cat-file", "blob", hash])
        .current_dir(repo_root)
        .output()
        .map_err(|e| format!("failed to read blob: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "blob not found: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// GET /api/v1/repos/:repo_id/sessions/:session_id/logs
async fn get_session_logs(
    State(state): State<DaemonState>,
    Path((repo_id, session_id)): Path<(String, String)>,
    Query(query): Query<SessionLogsQuery>,
) -> Result<Json<SessionLogsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let ctx = state.get_repo(&repo_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Repository not found: {}", repo_id),
            }),
        )
    })?;

    // Find the session
    let sessions = ctx.gritee.session_list(None).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to list sessions: {}", e),
            }),
        )
    })?;

    let session = sessions
        .into_iter()
        .find(|s| s.session_id == session_id)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Session not found: {}", session_id),
                }),
            )
        })?;

    // Check if there's log output
    let output_ref = session.last_output_ref.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "No logs available for this session".to_string(),
            }),
        )
    })?;

    // Read the blob
    let content = read_blob(&ctx.path, &output_ref).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to read logs: {}", e),
            }),
        )
    })?;

    // Split into lines and take last N
    let all_lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    let total_lines = all_lines.len();
    let start_idx = total_lines.saturating_sub(query.lines);
    let lines: Vec<String> = all_lines[start_idx..].to_vec();
    let has_more = start_idx > 0;

    Ok(Json(SessionLogsResponse { lines, has_more }))
}

/// Build session routes.
pub fn routes() -> Router<DaemonState> {
    Router::new()
        .route("/sessions", get(list_sessions))
        .route("/sessions/:session_id", get(get_session))
        .route("/sessions/:session_id/stop", post(stop_session))
        .route("/sessions/:session_id/logs", get(get_session_logs))
}
