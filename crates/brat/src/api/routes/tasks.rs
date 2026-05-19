//! Task CRUD endpoints.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use libbrat_grite::TaskStatus;
use serde::{Deserialize, Serialize};

use crate::api::state::DaemonState;

use super::status::ErrorResponse;

/// Task response.
#[derive(Serialize)]
pub struct TaskResponse {
    pub task_id: String,
    pub gritee_issue_id: String,
    pub convoy_id: String,
    pub title: String,
    pub body: String,
    pub status: String,
}

/// Query parameters for listing tasks.
#[derive(Deserialize, Default)]
pub struct ListTasksQuery {
    /// Filter by convoy ID.
    pub convoy: Option<String>,
    /// Filter by status.
    pub status: Option<String>,
}

/// Request to create a task.
#[derive(Deserialize)]
pub struct CreateTaskRequest {
    pub convoy_id: String,
    pub title: String,
    #[serde(default)]
    pub body: String,
}

/// Request to update task status.
#[derive(Deserialize)]
pub struct UpdateTaskRequest {
    pub status: String,
}

fn task_status_to_string(status: TaskStatus) -> String {
    match status {
        TaskStatus::Queued => "queued".to_string(),
        TaskStatus::Running => "running".to_string(),
        TaskStatus::Blocked => "blocked".to_string(),
        TaskStatus::NeedsReview => "needs-review".to_string(),
        TaskStatus::Merged => "merged".to_string(),
        TaskStatus::Dropped => "dropped".to_string(),
    }
}

fn parse_task_status(s: &str) -> Option<TaskStatus> {
    match s {
        "queued" => Some(TaskStatus::Queued),
        "running" => Some(TaskStatus::Running),
        "blocked" => Some(TaskStatus::Blocked),
        "needs-review" => Some(TaskStatus::NeedsReview),
        "merged" => Some(TaskStatus::Merged),
        "dropped" => Some(TaskStatus::Dropped),
        _ => None,
    }
}

/// GET /api/v1/repos/:repo_id/tasks
async fn list_tasks(
    State(state): State<DaemonState>,
    Path(repo_id): Path<String>,
    Query(query): Query<ListTasksQuery>,
) -> Result<Json<Vec<TaskResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let ctx = state.get_repo(&repo_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Repository not found: {}", repo_id),
            }),
        )
    })?;

    let tasks = ctx.gritee.task_list(query.convoy.as_deref()).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to list tasks: {}", e),
            }),
        )
    })?;

    let mut responses: Vec<TaskResponse> = tasks
        .into_iter()
        .map(|t| TaskResponse {
            task_id: t.task_id,
            gritee_issue_id: t.gritee_issue_id,
            convoy_id: t.convoy_id,
            title: t.title,
            body: t.body,
            status: task_status_to_string(t.status),
        })
        .collect();

    // Filter by status if specified
    if let Some(status_filter) = query.status {
        responses.retain(|t| t.status == status_filter);
    }

    Ok(Json(responses))
}

/// POST /api/v1/repos/:repo_id/tasks
async fn create_task(
    State(state): State<DaemonState>,
    Path(repo_id): Path<String>,
    Json(req): Json<CreateTaskRequest>,
) -> Result<(StatusCode, Json<TaskResponse>), (StatusCode, Json<ErrorResponse>)> {
    let ctx = state.get_repo(&repo_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Repository not found: {}", repo_id),
            }),
        )
    })?;

    let body = if req.body.is_empty() {
        None
    } else {
        Some(req.body.as_str())
    };

    let task = ctx
        .gritee
        .task_create(&req.convoy_id, &req.title, body)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to create task: {}", e),
                }),
            )
        })?;

    Ok((
        StatusCode::CREATED,
        Json(TaskResponse {
            task_id: task.task_id,
            gritee_issue_id: task.gritee_issue_id,
            convoy_id: task.convoy_id,
            title: task.title,
            body: task.body,
            status: task_status_to_string(task.status),
        }),
    ))
}

/// GET /api/v1/repos/:repo_id/tasks/:task_id
async fn get_task(
    State(state): State<DaemonState>,
    Path((repo_id, task_id)): Path<(String, String)>,
) -> Result<Json<TaskResponse>, (StatusCode, Json<ErrorResponse>)> {
    let ctx = state.get_repo(&repo_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Repository not found: {}", repo_id),
            }),
        )
    })?;

    // List tasks and find the one with matching ID
    let tasks = ctx.gritee.task_list(None).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to list tasks: {}", e),
            }),
        )
    })?;

    let task = tasks
        .into_iter()
        .find(|t| t.task_id == task_id)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Task not found: {}", task_id),
                }),
            )
        })?;

    Ok(Json(TaskResponse {
        task_id: task.task_id,
        gritee_issue_id: task.gritee_issue_id,
        convoy_id: task.convoy_id,
        title: task.title,
        body: task.body,
        status: task_status_to_string(task.status),
    }))
}

/// PATCH /api/v1/repos/:repo_id/tasks/:task_id
async fn update_task(
    State(state): State<DaemonState>,
    Path((repo_id, task_id)): Path<(String, String)>,
    Json(req): Json<UpdateTaskRequest>,
) -> Result<Json<TaskResponse>, (StatusCode, Json<ErrorResponse>)> {
    let ctx = state.get_repo(&repo_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Repository not found: {}", repo_id),
            }),
        )
    })?;

    let new_status = parse_task_status(&req.status).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("Invalid status: {}", req.status),
            }),
        )
    })?;

    ctx.gritee
        .task_update_status(&task_id, new_status)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to update task: {}", e),
                }),
            )
        })?;

    // Fetch the updated task
    let tasks = ctx.gritee.task_list(None).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to fetch updated task: {}", e),
            }),
        )
    })?;

    let task = tasks
        .into_iter()
        .find(|t| t.task_id == task_id)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Task not found after update: {}", task_id),
                }),
            )
        })?;

    Ok(Json(TaskResponse {
        task_id: task.task_id,
        gritee_issue_id: task.gritee_issue_id,
        convoy_id: task.convoy_id,
        title: task.title,
        body: task.body,
        status: task_status_to_string(task.status),
    }))
}

/// Build task routes.
pub fn routes() -> Router<DaemonState> {
    Router::new()
        .route("/tasks", get(list_tasks).post(create_task))
        .route("/tasks/:task_id", get(get_task).patch(update_task))
}
