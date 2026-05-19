//! Meta API endpoints.
//!
//! Provides HTTP endpoints for interacting with the AI Meta orchestrator.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use libbrat_engine::{Engine, MetaEngine, SpawnSpec};
use serde::{Deserialize, Serialize};

use crate::api::state::DaemonState;

use super::status::ErrorResponse;

/// Meta status response.
#[derive(Serialize)]
pub struct MetaStatusResponse {
    /// Whether the Meta is currently active.
    pub active: bool,
    /// Session ID if active.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

/// Request to start the Meta.
#[derive(Deserialize, Default)]
pub struct StartMetaRequest {
    /// Optional initial message.
    pub message: Option<String>,
}

/// Response from starting the Meta.
#[derive(Serialize)]
pub struct StartMetaResponse {
    /// Session ID.
    pub session_id: String,
    /// Initial response lines.
    pub response: Vec<String>,
}

/// Request to ask the Meta a question.
#[derive(Deserialize)]
pub struct AskMetaRequest {
    /// Message to send.
    pub message: String,
}

/// Response from asking the Meta.
#[derive(Serialize)]
pub struct AskMetaResponse {
    /// Response lines from the Meta.
    pub response: Vec<String>,
}

/// Response from stopping the Meta.
#[derive(Serialize)]
pub struct StopMetaResponse {
    /// Whether the stop was successful.
    pub success: bool,
}

/// Query parameters for getting Meta history.
#[derive(Deserialize, Default)]
pub struct MetaHistoryQuery {
    /// Number of lines to return (default: 50).
    #[serde(default = "default_history_lines")]
    pub lines: usize,
}

fn default_history_lines() -> usize {
    50
}

/// Response with Meta conversation history.
#[derive(Serialize)]
pub struct MetaHistoryResponse {
    /// Conversation history lines.
    pub lines: Vec<String>,
}

/// GET /api/v1/repos/:repo_id/meta/status
async fn get_meta_status(
    State(state): State<DaemonState>,
    Path(repo_id): Path<String>,
) -> Result<Json<MetaStatusResponse>, (StatusCode, Json<ErrorResponse>)> {
    let ctx = state.get_repo(&repo_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Repository not found: {}", repo_id),
            }),
        )
    })?;

    let engine = MetaEngine::new(ctx.path.clone());
    let active = engine.is_active();
    let session_id = engine.current_session_id();

    Ok(Json(MetaStatusResponse { active, session_id }))
}

/// POST /api/v1/repos/:repo_id/meta/start
async fn start_meta(
    State(state): State<DaemonState>,
    Path(repo_id): Path<String>,
    Json(req): Json<StartMetaRequest>,
) -> Result<(StatusCode, Json<StartMetaResponse>), (StatusCode, Json<ErrorResponse>)> {
    let ctx = state.get_repo(&repo_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Repository not found: {}", repo_id),
            }),
        )
    })?;

    let engine = MetaEngine::new(ctx.path.clone());

    // Check if already active
    if engine.is_active() {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: "Meta session already active - stop it first".to_string(),
            }),
        ));
    }

    // Create spawn spec
    let spec = SpawnSpec::new(req.message.unwrap_or_default())
        .working_dir(ctx.path.clone());

    // Spawn the Meta (this is async)
    let result = engine.spawn(spec).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to start Meta: {}", e),
            }),
        )
    })?;

    // Get initial response from history
    let response = engine.tail(50).unwrap_or_default();

    Ok((
        StatusCode::CREATED,
        Json(StartMetaResponse {
            session_id: result.session_id,
            response,
        }),
    ))
}

/// POST /api/v1/repos/:repo_id/meta/stop
async fn stop_meta(
    State(state): State<DaemonState>,
    Path(repo_id): Path<String>,
) -> Result<Json<StopMetaResponse>, (StatusCode, Json<ErrorResponse>)> {
    let ctx = state.get_repo(&repo_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Repository not found: {}", repo_id),
            }),
        )
    })?;

    let engine = MetaEngine::new(ctx.path.clone());

    engine.stop_session().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to stop Meta: {}", e),
            }),
        )
    })?;

    Ok(Json(StopMetaResponse { success: true }))
}

/// POST /api/v1/repos/:repo_id/meta/ask
async fn ask_meta(
    State(state): State<DaemonState>,
    Path(repo_id): Path<String>,
    Json(req): Json<AskMetaRequest>,
) -> Result<Json<AskMetaResponse>, (StatusCode, Json<ErrorResponse>)> {
    let ctx = state.get_repo(&repo_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Repository not found: {}", repo_id),
            }),
        )
    })?;

    let engine = MetaEngine::new(ctx.path.clone());

    // Check if active
    if !engine.is_active() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Meta not active - start it first".to_string(),
            }),
        ));
    }

    let response = engine.ask(&req.message).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to send message to Meta: {}", e),
            }),
        )
    })?;

    Ok(Json(AskMetaResponse { response }))
}

/// GET /api/v1/repos/:repo_id/meta/history
async fn get_meta_history(
    State(state): State<DaemonState>,
    Path(repo_id): Path<String>,
    Query(query): Query<MetaHistoryQuery>,
) -> Result<Json<MetaHistoryResponse>, (StatusCode, Json<ErrorResponse>)> {
    let ctx = state.get_repo(&repo_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Repository not found: {}", repo_id),
            }),
        )
    })?;

    let engine = MetaEngine::new(ctx.path.clone());

    let lines = engine.tail(query.lines).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to get Meta history: {}", e),
            }),
        )
    })?;

    Ok(Json(MetaHistoryResponse { lines }))
}

/// Build Meta routes.
pub fn routes() -> Router<DaemonState> {
    Router::new()
        .route("/meta/status", get(get_meta_status))
        .route("/meta/start", post(start_meta))
        .route("/meta/stop", post(stop_meta))
        .route("/meta/ask", post(ask_meta))
        .route("/meta/history", get(get_meta_history))
}
