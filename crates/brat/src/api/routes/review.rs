//! Review / approval gate API endpoints.
//!
//! Provides HTTP endpoints for human-in-the-loop merge approval.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::api::state::DaemonState;

use super::status::ErrorResponse;

// ------------------------------------------------------------------
// Pending approvals
// ------------------------------------------------------------------

/// An approval request.
#[derive(Serialize)]
pub struct ApprovalRequest {
    pub task_id: String,
    pub convoy_id: String,
    pub title: String,
    pub description: String,
    pub diff_summary: String,
    pub test_status: String,
    pub risk_level: String,
}

/// GET /api/v1/repos/:repo_id/review/pending
async fn review_pending(
    State(state): State<DaemonState>,
    Path(repo_id): Path<String>,
) -> Result<Json<Vec<ApprovalRequest>>, (StatusCode, Json<ErrorResponse>)> {
    let _ctx = state.get_repo(&repo_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Repository not found: {}", repo_id),
            }),
        )
    })?;

    // TODO: Integrate with refinery workflow
    Ok(Json(vec![]))
}

// ------------------------------------------------------------------
// Approve
// ------------------------------------------------------------------

/// Request to approve a merge.
#[derive(Deserialize)]
pub struct ApproveRequest {
    /// Optional review comment.
    pub comment: Option<String>,
}

/// Approval response.
#[derive(Serialize)]
pub struct ApproveResponse {
    pub success: bool,
    pub task_id: String,
    pub approved: bool,
}

/// POST /api/v1/repos/:repo_id/review/:task_id/approve
async fn review_approve(
    State(state): State<DaemonState>,
    Path((repo_id, task_id)): Path<(String, String)>,
    Json(_req): Json<ApproveRequest>,
) -> Result<Json<ApproveResponse>, (StatusCode, Json<ErrorResponse>)> {
    let _ctx = state.get_repo(&repo_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Repository not found: {}", repo_id),
            }),
        )
    })?;

    // TODO: Integrate with refinery workflow
    Ok(Json(ApproveResponse {
        success: true,
        task_id,
        approved: true,
    }))
}

// ------------------------------------------------------------------
// Reject / request changes
// ------------------------------------------------------------------

/// Request to reject a merge.
#[derive(Deserialize)]
pub struct RejectRequest {
    pub reason: String,
}

/// Rejection response.
#[derive(Serialize)]
pub struct RejectResponse {
    pub success: bool,
    pub task_id: String,
    pub rejected: bool,
}

/// POST /api/v1/repos/:repo_id/review/:task_id/reject
async fn review_reject(
    State(state): State<DaemonState>,
    Path((repo_id, task_id)): Path<(String, String)>,
    Json(req): Json<RejectRequest>,
) -> Result<Json<RejectResponse>, (StatusCode, Json<ErrorResponse>)> {
    let _ctx = state.get_repo(&repo_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Repository not found: {}", repo_id),
            }),
        )
    })?;

    // TODO: Integrate with refinery workflow
    Ok(Json(RejectResponse {
        success: true,
        task_id,
        rejected: true,
    }))
}

/// Build review routes.
pub fn routes() -> Router<DaemonState> {
    Router::new()
        .route("/review/pending", get(review_pending))
        .route("/review/:task_id/approve", post(review_approve))
        .route("/review/:task_id/reject", post(review_reject))
}
