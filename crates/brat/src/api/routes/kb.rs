//! Knowledge Base API endpoints.
//!
//! Provides HTTP endpoints for reading product/architecture notes,
//! consistency scores, and inconsistencies. Writes go through the
//! Meta agent or CLI.
//!
//! All zkb operations run inside `tokio::task::spawn_blocking` because
//! `zkb_lib::Zkb` is `!Sync` (contains `RefCell`/`rusqlite`). This keeps the
//! axum state `Send + Sync`.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use crate::api::state::DaemonState;

use super::status::ErrorResponse;

/// Run a closure on a blocking thread where `!Sync` zkb types are safe.
async fn run_kb<F, R>(f: F) -> R
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .expect("kb task panicked")
}

// ------------------------------------------------------------------
// Search
// ------------------------------------------------------------------

/// Query parameters for KB search.
#[derive(Deserialize, Default)]
pub struct KbSearchQuery {
    /// Search query string.
    pub q: String,
    /// Optional note type filter.
    pub note_type: Option<String>,
}

/// A search result item.
#[derive(Serialize)]
pub struct KbSearchResult {
    pub slug: String,
    pub title: String,
    pub note_type: String,
    pub score: f64,
}

/// Search response.
#[derive(Serialize)]
pub struct KbSearchResponse {
    pub query: String,
    pub results: Vec<KbSearchResult>,
}

/// GET /api/v1/repos/:repo_id/kb/search
async fn kb_search(
    State(state): State<DaemonState>,
    Path(repo_id): Path<String>,
    Query(query): Query<KbSearchQuery>,
) -> Result<Json<KbSearchResponse>, (StatusCode, Json<ErrorResponse>)> {
    let ctx = state.get_repo(&repo_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Repository not found: {}", repo_id),
            }),
        )
    })?;

    let path = ctx.path.clone();
    let q = query.q.clone();

    let results = run_kb(move || {
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async {
            let kb = libbrat_kb::KbService::open(&path)
                .map_err(|e| format!("KB open failed: {}", e))?;
            kb.search(&q)
                .await
                .map_err(|e| format!("Search failed: {}", e))
        })
    })
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e,
            }),
        )
    })?;

    Ok(Json(KbSearchResponse {
        query: query.q,
        results: results
            .into_iter()
            .map(|r| KbSearchResult {
                slug: r.slug,
                title: r.title,
                note_type: r.note_type,
                score: r.score,
            })
            .collect(),
    }))
}

// ------------------------------------------------------------------
// Product notes
// ------------------------------------------------------------------

/// A product note summary.
#[derive(Serialize)]
pub struct ProductNoteSummary {
    pub slug: String,
    pub title: String,
    pub priority: String,
}

/// GET /api/v1/repos/:repo_id/kb/product
async fn kb_product(
    State(state): State<DaemonState>,
    Path(repo_id): Path<String>,
) -> Result<Json<Vec<ProductNoteSummary>>, (StatusCode, Json<ErrorResponse>)> {
    let ctx = state.get_repo(&repo_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Repository not found: {}", repo_id),
            }),
        )
    })?;

    let path = ctx.path.clone();

    let notes = run_kb(move || {
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async {
            let kb = libbrat_kb::KbService::open(&path)
                .map_err(|e| format!("KB open failed: {}", e))?;
            kb.list_product_notes()
                .await
                .map_err(|e| format!("List product notes failed: {}", e))
        })
    })
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e,
            }),
        )
    })?;

    Ok(Json(
        notes
            .into_iter()
            .map(|n| ProductNoteSummary {
                slug: n.slug,
                title: n.title,
                priority: n.priority,
            })
            .collect(),
    ))
}

// ------------------------------------------------------------------
// Architecture notes
// ------------------------------------------------------------------

/// An architecture note summary.
#[derive(Serialize)]
pub struct ArchitectureNoteSummary {
    pub slug: String,
    pub title: String,
    pub component_count: usize,
}

/// GET /api/v1/repos/:repo_id/kb/architecture
async fn kb_architecture(
    State(state): State<DaemonState>,
    Path(repo_id): Path<String>,
) -> Result<Json<Vec<ArchitectureNoteSummary>>, (StatusCode, Json<ErrorResponse>)> {
    let ctx = state.get_repo(&repo_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Repository not found: {}", repo_id),
            }),
        )
    })?;

    let path = ctx.path.clone();

    let notes = run_kb(move || {
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async {
            let kb = libbrat_kb::KbService::open(&path)
                .map_err(|e| format!("KB open failed: {}", e))?;
            kb.list_architecture_notes()
                .await
                .map_err(|e| format!("List architecture notes failed: {}", e))
        })
    })
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e,
            }),
        )
    })?;

    Ok(Json(
        notes
            .into_iter()
            .map(|n| ArchitectureNoteSummary {
                slug: n.slug,
                title: n.title,
                component_count: n.component_count,
            })
            .collect(),
    ))
}

// ------------------------------------------------------------------
// Consistency score
// ------------------------------------------------------------------

/// Consistency score response.
#[derive(Serialize)]
pub struct ConsistencyScoreResponse {
    pub score: u8,
    pub product_arch_coverage: f64,
    pub arch_product_traceability: f64,
    pub file_component_mapping: f64,
    pub test_feature_coverage: f64,
    pub doc_component_parity: f64,
}

/// GET /api/v1/repos/:repo_id/kb/score
async fn kb_score(
    State(state): State<DaemonState>,
    Path(repo_id): Path<String>,
) -> Result<Json<ConsistencyScoreResponse>, (StatusCode, Json<ErrorResponse>)> {
    let ctx = state.get_repo(&repo_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Repository not found: {}", repo_id),
            }),
        )
    })?;

    let path = ctx.path.clone();

    let check = run_kb(move || {
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async {
            let kb = libbrat_kb::KbService::open(&path)
                .map_err(|e| format!("KB open failed: {}", e))?;
            kb.run_consistency_check()
                .await
                .map_err(|e| format!("Consistency check failed: {}", e))
        })
    })
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e,
            }),
        )
    })?;

    Ok(Json(ConsistencyScoreResponse {
        score: check.score(),
        product_arch_coverage: check.product_arch_coverage,
        arch_product_traceability: check.arch_product_traceability,
        file_component_mapping: check.file_component_mapping,
        test_feature_coverage: check.test_feature_coverage,
        doc_component_parity: check.doc_component_parity,
    }))
}

// ------------------------------------------------------------------
// Inconsistencies
// ------------------------------------------------------------------

/// An inconsistency item.
#[derive(Serialize)]
pub struct InconsistencyItem {
    pub kind: String,
    pub severity: String,
    pub description: String,
    pub suggested_fix: String,
}

/// GET /api/v1/repos/:repo_id/kb/inconsistencies
async fn kb_inconsistencies(
    State(state): State<DaemonState>,
    Path(repo_id): Path<String>,
) -> Result<Json<Vec<InconsistencyItem>>, (StatusCode, Json<ErrorResponse>)> {
    let ctx = state.get_repo(&repo_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Repository not found: {}", repo_id),
            }),
        )
    })?;

    let path = ctx.path.clone();

    let inconsistencies = run_kb(move || {
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async {
            let kb = libbrat_kb::KbService::open(&path)
                .map_err(|e| format!("KB open failed: {}", e))?;
            kb.list_inconsistencies()
                .await
                .map_err(|e| format!("List inconsistencies failed: {}", e))
        })
    })
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e,
            }),
        )
    })?;

    Ok(Json(
        inconsistencies
            .into_iter()
            .map(|i| InconsistencyItem {
                kind: format!("{:?}", i.kind),
                severity: format!("{:?}", i.severity),
                description: i.description,
                suggested_fix: i.suggested_fix,
            })
            .collect(),
    ))
}

// ------------------------------------------------------------------
// Sync from filesystem
// ------------------------------------------------------------------

/// Sync response.
#[derive(Serialize)]
pub struct SyncResponse {
    pub strategy: String,
    pub drifted: Vec<String>,
    pub reconciled: Vec<String>,
    pub errors: Vec<(String, String)>,
}

/// POST /api/v1/repos/:repo_id/kb/sync
async fn kb_sync(
    State(state): State<DaemonState>,
    Path(repo_id): Path<String>,
) -> Result<Json<SyncResponse>, (StatusCode, Json<ErrorResponse>)> {
    let ctx = state.get_repo(&repo_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Repository not found: {}", repo_id),
            }),
        )
    })?;

    let path = ctx.path.clone();

    let output = run_kb(move || {
        let kb = libbrat_kb::KbService::open(&path)
            .map_err(|e| format!("KB open failed: {}", e))?;
        kb.sync_from_fs()
            .map_err(|e| format!("Sync failed: {}", e))
    })
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e,
            }),
        )
    })?;

    Ok(Json(SyncResponse {
        strategy: output.strategy,
        drifted: output.drifted,
        reconciled: output.reconciled,
        errors: output.errors,
    }))
}

/// Build KB routes.
pub fn routes() -> Router<DaemonState> {
    Router::new()
        .route("/kb/search", get(kb_search))
        .route("/kb/product", get(kb_product))
        .route("/kb/architecture", get(kb_architecture))
        .route("/kb/score", get(kb_score))
        .route("/kb/inconsistencies", get(kb_inconsistencies))
        .route("/kb/sync", post(kb_sync))
}
