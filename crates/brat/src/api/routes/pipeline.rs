//! Pipeline status endpoints.

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;

use crate::api::state::DaemonState;

/// Phase status response.
#[derive(Serialize)]
pub struct PhaseStatusResponse {
    pub phase: String,
    pub status: String,
    pub notes_created: usize,
    pub gate_status: String,
}

/// GET /api/v1/repos/:repo_id/pipeline - Get pipeline phase statuses.
async fn get_pipeline(State(_state): State<DaemonState>) -> Json<Vec<PhaseStatusResponse>> {
    // Stub: return placeholder pipeline data.
    Json(vec![
        PhaseStatusResponse {
            phase: "product".to_string(),
            status: "complete".to_string(),
            notes_created: 5,
            gate_status: "open".to_string(),
        },
        PhaseStatusResponse {
            phase: "architecture".to_string(),
            status: "in_progress".to_string(),
            notes_created: 3,
            gate_status: "open".to_string(),
        },
        PhaseStatusResponse {
            phase: "implementation".to_string(),
            status: "pending".to_string(),
            notes_created: 0,
            gate_status: "closed".to_string(),
        },
        PhaseStatusResponse {
            phase: "review".to_string(),
            status: "pending".to_string(),
            notes_created: 0,
            gate_status: "closed".to_string(),
        },
        PhaseStatusResponse {
            phase: "merge".to_string(),
            status: "pending".to_string(),
            notes_created: 0,
            gate_status: "closed".to_string(),
        },
        PhaseStatusResponse {
            phase: "memory".to_string(),
            status: "pending".to_string(),
            notes_created: 0,
            gate_status: "closed".to_string(),
        },
    ])
}

/// Build pipeline routes.
pub fn routes() -> Router<DaemonState> {
    Router::new().route("/pipeline", get(get_pipeline))
}
