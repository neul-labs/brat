//! Shared state for the bratd daemon.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use libbrat_config::BratConfig;
use libbrat_grite::GriteeClient;
use libbrat_worktree::WorktreeManager;
use serde::Serialize;
use tokio::sync::{broadcast, RwLock};

/// Events broadcast to WebSocket clients for real-time updates.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", content = "data")]
pub enum BratEvent {
    /// Task status changed.
    TaskUpdated {
        task_id: String,
        status: String,
        convoy_id: Option<String>,
    },
    /// New session started.
    SessionStarted {
        session_id: String,
        task_id: String,
        engine: String,
    },
    /// Session exited (completed or failed).
    SessionExited {
        session_id: String,
        task_id: String,
        exit_code: i32,
    },
    /// Merge completed successfully.
    MergeCompleted {
        task_id: String,
        commit_sha: String,
        branch: String,
    },
    /// Merge failed.
    MergeFailed {
        task_id: String,
        error: String,
        attempt: u32,
    },
    /// Merge was rolled back after push failure.
    MergeRolledBack {
        task_id: String,
        reset_sha: String,
        reason: String,
    },
    /// Merge scheduled for retry.
    MergeRetryScheduled {
        task_id: String,
        retry_at: String,
        attempt: u32,
    },
    /// KB mirror synced from filesystem.
    KbSynced {
        repo_id: String,
        drifted: Vec<String>,
        reconciled: Vec<String>,
    },
}

/// Broadcast channel capacity for events.
const EVENT_CHANNEL_CAPACITY: usize = 256;

/// Default idle timeout in seconds (15 minutes).
pub const DEFAULT_IDLE_TIMEOUT_SECS: u64 = 900;

/// Global daemon state shared across all request handlers.
#[derive(Clone)]
pub struct DaemonState {
    /// Registry of known repositories.
    pub repos: Arc<RwLock<HashMap<String, Arc<RepoContext>>>>,
    /// Daemon start time for uptime calculation.
    pub start_time: Instant,
    /// Last activity timestamp (updated on each request).
    pub last_activity: Arc<RwLock<Instant>>,
    /// Idle timeout duration. If no requests for this long, daemon shuts down.
    /// None means no idle timeout (run forever).
    pub idle_timeout: Option<Duration>,
    /// Version string.
    pub version: String,
    /// Broadcast channel for real-time events to WebSocket clients.
    event_tx: broadcast::Sender<BratEvent>,
}

impl DaemonState {
    /// Create new daemon state with optional idle timeout.
    pub fn new(idle_timeout_secs: Option<u64>) -> Self {
        let (event_tx, _) = broadcast::channel(EVENT_CHANNEL_CAPACITY);
        Self {
            repos: Arc::new(RwLock::new(HashMap::new())),
            start_time: Instant::now(),
            last_activity: Arc::new(RwLock::new(Instant::now())),
            idle_timeout: idle_timeout_secs.map(Duration::from_secs),
            version: env!("CARGO_PKG_VERSION").to_string(),
            event_tx,
        }
    }

    /// Broadcast an event to all connected WebSocket clients.
    pub fn broadcast(&self, event: BratEvent) {
        // Ignore send errors - they just mean no clients are connected
        let _ = self.event_tx.send(event);
    }

    /// Subscribe to events (for WebSocket handlers).
    pub fn subscribe_events(&self) -> broadcast::Receiver<BratEvent> {
        self.event_tx.subscribe()
    }

    /// Record activity (call this on each request).
    pub async fn touch(&self) {
        let mut last = self.last_activity.write().await;
        *last = Instant::now();
    }

    /// Get seconds since last activity.
    pub async fn idle_secs(&self) -> u64 {
        let last = self.last_activity.read().await;
        last.elapsed().as_secs()
    }

    /// Check if idle timeout has been exceeded.
    pub async fn is_idle_timeout_exceeded(&self) -> bool {
        if let Some(timeout) = self.idle_timeout {
            let last = self.last_activity.read().await;
            last.elapsed() > timeout
        } else {
            false
        }
    }

    /// Get uptime in seconds.
    pub fn uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// Register a repository.
    pub async fn register_repo(&self, path: PathBuf) -> Result<Arc<RepoContext>, String> {
        // Validate the path is a git repo with brat initialized
        let git_dir = path.join(".git");
        if !git_dir.exists() {
            return Err(format!("Not a git repository: {:?}", path));
        }

        let brat_dir = path.join(".brat");
        let config_path = brat_dir.join("config.toml");
        if !config_path.exists() {
            return Err(format!("Brat not initialized in: {:?}", path));
        }

        // Load config
        let config = BratConfig::load(&config_path)
            .map_err(|e| format!("Failed to load config: {}", e))?;

        // Create Gritee client
        let gritee = GriteeClient::new(&path);

        // Create worktree manager
        let worktree_manager = WorktreeManager::new(
            &path,
            &config.swarm.worktree_root,
            config.swarm.max_polecats,
        );

        let repo_id = path_to_repo_id(&path);
        let context = Arc::new(RepoContext {
            id: repo_id.clone(),
            path: path.clone(),
            gritee,
            config,
            worktree_manager: Some(worktree_manager),
        });

        let mut repos = self.repos.write().await;
        repos.insert(repo_id, Arc::clone(&context));

        Ok(context)
    }

    /// Get a repository by ID.
    pub async fn get_repo(&self, repo_id: &str) -> Option<Arc<RepoContext>> {
        let repos = self.repos.read().await;
        repos.get(repo_id).cloned()
    }

    /// Unregister a repository.
    pub async fn unregister_repo(&self, repo_id: &str) -> bool {
        let mut repos = self.repos.write().await;
        repos.remove(repo_id).is_some()
    }

    /// List all registered repositories.
    pub async fn list_repos(&self) -> Vec<Arc<RepoContext>> {
        let repos = self.repos.read().await;
        repos.values().cloned().collect()
    }
}

impl Default for DaemonState {
    fn default() -> Self {
        Self::new(Some(DEFAULT_IDLE_TIMEOUT_SECS))
    }
}

/// Context for a single repository.
pub struct RepoContext {
    /// Repository ID (base64 encoded path or short ID).
    pub id: String,
    /// Path to repository root.
    pub path: PathBuf,
    /// Gritee client for this repo.
    pub gritee: GriteeClient,
    /// Brat configuration.
    pub config: BratConfig,
    /// Worktree manager (if available).
    pub worktree_manager: Option<WorktreeManager>,
}

/// Convert a path to a repo ID (base64 encoded).
pub fn path_to_repo_id(path: &PathBuf) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(path.to_string_lossy().as_bytes())
}

/// Convert a repo ID back to a path.
#[allow(dead_code)]
pub fn repo_id_to_path(repo_id: &str) -> Result<PathBuf, String> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(repo_id)
        .map_err(|e| format!("Invalid repo ID: {}", e))?;
    let path_str = String::from_utf8(bytes)
        .map_err(|e| format!("Invalid UTF-8 in repo ID: {}", e))?;
    Ok(PathBuf::from(path_str))
}
