//! Axum HTTP server setup.

use std::net::SocketAddr;
use std::time::Duration;

use axum::body::Body;
use axum::extract::State;
use axum::http::Request;
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::Router;
use tokio::sync::watch;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;

use super::routes;
use super::state::{DaemonState, DEFAULT_IDLE_TIMEOUT_SECS};

/// Server configuration.
pub struct ServerConfig {
    /// Host to bind to.
    pub host: String,
    /// Port to listen on.
    pub port: u16,
    /// CORS allowed origin (None = allow all).
    pub cors_origin: Option<String>,
    /// Idle timeout in seconds. None means no timeout (run forever).
    /// 0 also means no timeout.
    pub idle_timeout_secs: Option<u64>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3000,
            cors_origin: None,
            idle_timeout_secs: Some(DEFAULT_IDLE_TIMEOUT_SECS),
        }
    }
}

/// Middleware to track activity for idle timeout.
async fn activity_tracker(
    State(state): State<DaemonState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    state.touch().await;
    next.run(request).await
}

/// Build the Axum router with all routes.
pub fn build_router(state: DaemonState) -> Router {
    // Build CORS layer
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .nest("/api/v1", routes::api_routes())
        .layer(middleware::from_fn_with_state(state.clone(), activity_tracker))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Run the HTTP server.
pub async fn run_server(config: ServerConfig) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "brat=info,tower_http=info".into()),
        )
        .init();

    // Normalize idle timeout (0 means no timeout)
    let idle_timeout_secs = config.idle_timeout_secs.filter(|&t| t > 0);

    // Create daemon state
    let state = DaemonState::new(idle_timeout_secs);

    // Build router
    let app = build_router(state.clone());

    // Parse address
    let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;

    if let Some(timeout) = idle_timeout_secs {
        info!(
            "Starting bratd on http://{} (idle timeout: {}s)",
            addr, timeout
        );
    } else {
        info!("Starting bratd on http://{} (no idle timeout)", addr);
    }

    // Create shutdown signal channel
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // Spawn idle checker task if timeout is configured
    if idle_timeout_secs.is_some() {
        let state_clone = state.clone();
        let shutdown_tx_clone = shutdown_tx.clone();
        tokio::spawn(async move {
            idle_shutdown_task(state_clone, shutdown_tx_clone).await;
        });
    }

    // Spawn KB filesystem watcher
    crate::api::watcher::spawn_kb_watchers(state.clone());

    // Run server with graceful shutdown
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(shutdown_rx))
        .await?;

    info!("bratd shut down");
    Ok(())
}

/// Task that monitors idle time and triggers shutdown.
async fn idle_shutdown_task(state: DaemonState, shutdown_tx: watch::Sender<bool>) {
    // Check every 30 seconds
    let check_interval = Duration::from_secs(30);

    loop {
        tokio::time::sleep(check_interval).await;

        if state.is_idle_timeout_exceeded().await {
            let idle_secs = state.idle_secs().await;
            info!(
                "Idle timeout exceeded ({}s idle), initiating shutdown",
                idle_secs
            );
            let _ = shutdown_tx.send(true);
            return;
        }
    }
}

/// Shutdown signal that waits for the watch channel or Ctrl+C.
async fn shutdown_signal(mut shutdown_rx: watch::Receiver<bool>) {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    let idle_shutdown = async {
        while !*shutdown_rx.borrow_and_update() {
            if shutdown_rx.changed().await.is_err() {
                // Channel closed, just wait forever
                std::future::pending::<()>().await;
            }
        }
    };

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C, shutting down");
        }
        _ = idle_shutdown => {
            info!("Idle shutdown triggered");
        }
    }
}
