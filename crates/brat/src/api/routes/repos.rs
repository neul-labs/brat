//! Repository management endpoints.

use std::path::PathBuf;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::api::state::{DaemonState, RepoContext};
use std::sync::Arc;

use super::bootstrap;
use super::convoys;
use super::kb;
use super::meta;
use super::pipeline;
use super::review;
use super::sessions;
use super::status;
use super::tasks;

/// Repository summary for list endpoint.
#[derive(Serialize)]
pub struct RepoSummary {
    /// Repository ID.
    pub id: String,
    /// Repository path.
    pub path: String,
    /// Repository name (last component of path).
    pub name: String,
}

impl From<&Arc<RepoContext>> for RepoSummary {
    fn from(ctx: &Arc<RepoContext>) -> Self {
        Self {
            id: ctx.id.clone(),
            path: ctx.path.to_string_lossy().to_string(),
            name: ctx
                .path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default(),
        }
    }
}

/// Request to register a repository.
#[derive(Deserialize)]
pub struct RegisterRepoRequest {
    /// Path to the repository.
    pub path: String,
}

/// Response for register endpoint.
#[derive(Serialize)]
pub struct RegisterRepoResponse {
    /// Whether registration succeeded.
    pub success: bool,
    /// Repository summary if successful.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo: Option<RepoSummary>,
    /// Error message if failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// GET /api/v1/repos - List all registered repositories.
async fn list_repos(State(state): State<DaemonState>) -> Json<Vec<RepoSummary>> {
    let repos = state.list_repos().await;
    Json(repos.iter().map(RepoSummary::from).collect())
}

/// POST /api/v1/repos - Register a repository.
async fn register_repo(
    State(state): State<DaemonState>,
    Json(req): Json<RegisterRepoRequest>,
) -> (StatusCode, Json<RegisterRepoResponse>) {
    let path = PathBuf::from(&req.path);

    match state.register_repo(path).await {
        Ok(ctx) => (
            StatusCode::CREATED,
            Json(RegisterRepoResponse {
                success: true,
                repo: Some(RepoSummary::from(&ctx)),
                error: None,
            }),
        ),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(RegisterRepoResponse {
                success: false,
                repo: None,
                error: Some(e),
            }),
        ),
    }
}

/// DELETE /api/v1/repos/:repo_id - Unregister a repository.
async fn unregister_repo(
    State(state): State<DaemonState>,
    Path(repo_id): Path<String>,
) -> StatusCode {
    if state.unregister_repo(&repo_id).await {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}

/// Build repos routes.
pub fn routes() -> Router<DaemonState> {
    Router::new()
        .route("/repos", get(list_repos).post(register_repo))
        .route("/repos/:repo_id", delete(unregister_repo))
        // Nested repo-scoped routes
        .nest("/repos/:repo_id", repo_scoped_routes())
}

/// Build repo-scoped routes (require valid repo_id).
fn repo_scoped_routes() -> Router<DaemonState> {
    Router::new()
        .merge(status::routes())
        .merge(convoys::routes())
        .merge(tasks::routes())
        .merge(sessions::routes())
        .merge(meta::routes())
        .merge(kb::routes())
        .merge(bootstrap::routes())
        .merge(review::routes())
        .merge(pipeline::routes())
}
