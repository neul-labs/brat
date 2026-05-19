//! Bootstrap API endpoints.
//!
//! Provides HTTP endpoints for triggering bootstrap and reading
//! bootstrap status/progress.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::api::state::DaemonState;

use super::status::ErrorResponse;

// ------------------------------------------------------------------
// Status
// ------------------------------------------------------------------

/// Bootstrap status response.
#[derive(Serialize)]
pub struct BootstrapStatusResponse {
    /// Whether bootstrap has been run.
    pub ran: bool,
    /// Whether product and architecture notes are consistent.
    pub consistent: bool,
    /// Consistency score (0-100).
    pub score: u8,
    /// Number of remaining inconsistencies.
    pub inconsistency_count: usize,
    /// Iterations run.
    pub iterations: u32,
}

/// GET /api/v1/repos/:repo_id/bootstrap/status
async fn bootstrap_status(
    State(state): State<DaemonState>,
    Path(repo_id): Path<String>,
) -> Result<Json<BootstrapStatusResponse>, (StatusCode, Json<ErrorResponse>)> {
    let _ctx = state.get_repo(&repo_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Repository not found: {}", repo_id),
            }),
        )
    })?;

    // TODO: Read bootstrap state from .brat/bootstrap_state.json
    Ok(Json(BootstrapStatusResponse {
        ran: false,
        consistent: false,
        score: 0,
        inconsistency_count: 0,
        iterations: 0,
    }))
}

// ------------------------------------------------------------------
// Trigger
// ------------------------------------------------------------------

/// Request to trigger bootstrap.
#[derive(Deserialize, Default)]
pub struct BootstrapRunRequest {
    /// Max iterations for consistency fix attempts.
    #[serde(default = "default_max_iterations")]
    pub max_iterations: u32,
}

fn default_max_iterations() -> u32 {
    5
}

/// Bootstrap run response.
#[derive(Serialize)]
pub struct BootstrapRunResponse {
    pub success: bool,
    pub consistent: bool,
    pub score: u8,
    pub inconsistency_count: usize,
    pub iterations: u32,
}

/// POST /api/v1/repos/:repo_id/bootstrap/run
async fn bootstrap_run(
    State(state): State<DaemonState>,
    Path(repo_id): Path<String>,
    Json(req): Json<BootstrapRunRequest>,
) -> Result<(StatusCode, Json<BootstrapRunResponse>), (StatusCode, Json<ErrorResponse>)> {
    let _ctx = state.get_repo(&repo_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Repository not found: {}", repo_id),
            }),
        )
    })?;

    // TODO: Trigger bootstrap via libbrat_engine::MetaEngine
    Ok((
        StatusCode::CREATED,
        Json(BootstrapRunResponse {
            success: true,
            consistent: false,
            score: 0,
            inconsistency_count: 0,
            iterations: 0,
        }),
    ))
}

/// Build bootstrap routes.
pub fn routes() -> Router<DaemonState> {
    Router::new()
        .route("/bootstrap/status", get(bootstrap_status))
        .route("/bootstrap/run", post(bootstrap_run))
}
