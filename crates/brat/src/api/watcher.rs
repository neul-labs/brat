//! Filesystem watcher for KB mirror auto-sync.
//!
//! Spawns a background task that watches `.brat/notes/` for each registered
//! repository, debounces events, and triggers `KbService::sync_from_fs()`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{mpsc, Arc};
use std::time::Duration;

use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::Mutex;
use tokio::time::interval;
use tracing::{debug, error, info, warn};

use crate::api::state::{BratEvent, DaemonState};

/// Spawn the KB filesystem watcher task.
pub fn spawn_kb_watchers(state: DaemonState) {
    tokio::spawn(watcher_task(state));
}

async fn watcher_task(state: DaemonState) {
    // Channel from notify callback into async task
    let (tx, mut rx) = tokio::sync::mpsc::channel::<( String, PathBuf)>(256);

    // Clone state for the watcher thread so we don't move the original
    let state_for_thread = state.clone();

    // Watchers are !Send in some notify backends, so keep them on a dedicated thread.
    let watcher_thread = std::thread::spawn(move || {
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                error!("Failed to create runtime for watcher: {}", e);
                return;
            }
        };

        let (notify_tx, notify_rx) = mpsc::channel::<notify::Result<Event>>();
        let mut watcher: RecommendedWatcher = match Watcher::new(
            move |res: notify::Result<Event>| {
                let _ = notify_tx.send(res);
            },
            Config::default().with_poll_interval(Duration::from_secs(2)),
        ) {
            Ok(w) => w,
            Err(e) => {
                error!("Failed to create notify watcher: {}", e);
                return;
            }
        };

        // Track which paths we are currently watching
        let mut watched: HashMap<String, PathBuf> = HashMap::new();
        let mut last_scan = std::time::Instant::now() - Duration::from_secs(10);

        loop {
            // Scan for new repos every 5 seconds
            if last_scan.elapsed() >= Duration::from_secs(5) {
                last_scan = std::time::Instant::now();
                let repos = rt.block_on(async { state_for_thread.list_repos().await });
                for repo in repos {
                    let notes_dir = repo.path.join(".brat").join("notes");
                    if notes_dir.exists() {
                        if !watched.values().any(|p| p == &notes_dir) {
                            match watcher.watch(&notes_dir, RecursiveMode::NonRecursive) {
                                Ok(_) => {
                                    let repo_id = repo.id.clone();
                                    info!(
                                        repo_id = %repo_id,
                                        path = %notes_dir.display(),
                                        "Watching KB notes directory"
                                    );
                                    watched.insert(repo_id, notes_dir);
                                }
                                Err(e) => {
                                    let repo_id = repo.id.clone();
                                    warn!(
                                        repo_id = %repo_id,
                                        path = %notes_dir.display(),
                                        "Failed to watch KB notes: {}", e
                                    );
                                }
                            }
                        }
                    }
                }

                // Remove watches for unregistered repos
                let active_ids: std::collections::HashSet<String> =
                    rt.block_on(async { state_for_thread.list_repos().await })
                        .into_iter()
                        .map(|r| r.id.clone())
                        .collect();
                let to_remove: Vec<String> = watched
                    .keys()
                    .filter(|k| !active_ids.contains(*k))
                    .cloned()
                    .collect();
                for id in to_remove {
                    if let Some(path) = watched.remove(&id) {
                        let _ = watcher.unwatch(&path);
                        debug!(repo_id = %id, "Stopped watching KB notes");
                    }
                }
            }

            // Process notify events with a debounce
            match notify_rx.recv_timeout(Duration::from_secs(1)) {
                Ok(Ok(event)) => {
                    if event.kind.is_modify() || event.kind.is_create() {
                        for path in &event.paths {
                            if let Some(ext) = path.extension() {
                                if ext == "md" {
                                    // Find which repo this path belongs to
                                    for (repo_id, notes_dir) in &watched {
                                        if path.starts_with(notes_dir) {
                                            let _ = tx.try_send((repo_id.clone(), notes_dir.clone()));
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(Err(e)) => {
                    warn!("Notify error: {}", e);
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    });

    // Debounce and sync in the async task
    let pending: Arc<Mutex<HashMap<String, PathBuf>>> = Arc::new(Mutex::new(HashMap::new()));

    // Task to receive events
    let pending_recv = pending.clone();
    let recv_task = tokio::spawn(async move {
        while let Some((repo_id, notes_dir)) = rx.recv().await {
            let mut map = pending_recv.lock().await;
            map.insert(repo_id, notes_dir);
        }
    });

    // Task to periodically flush pending syncs
    let pending_flush = pending.clone();
    let state_flush = state.clone();
    let flush_task = tokio::spawn(async move {
        let mut ticker = interval(Duration::from_millis(500));
        loop {
            ticker.tick().await;
            let to_sync: Vec<(String, PathBuf)> = {
                let mut map = pending_flush.lock().await;
                let items = map.drain().collect::<Vec<_>>();
                items
            };
            for (repo_id, notes_dir) in to_sync {
                let parent = notes_dir.parent().map(|p| p.to_path_buf());
                if let Some(repo_root) = parent {
                    let repo_id_clone = repo_id.clone();
                    let result = tokio::task::spawn_blocking(move || {
                        let kb = libbrat_kb::KbService::open(&repo_root)
                            .map_err(|e| format!("KB open failed: {}", e))?;
                        kb.sync_from_fs()
                            .map_err(|e| format!("Sync failed: {}", e))
                    })
                    .await;

                    match result {
                        Ok(Ok(output)) => {
                            info!(
                                repo_id = %repo_id_clone,
                                drifted = output.drifted.len(),
                                reconciled = output.reconciled.len(),
                                "KB auto-synced from filesystem"
                            );
                            let _ = state_flush.broadcast(BratEvent::KbSynced {
                                repo_id: repo_id_clone,
                                drifted: output.drifted,
                                reconciled: output.reconciled,
                            });
                        }
                        Ok(Err(e)) => {
                            warn!(repo_id = %repo_id_clone, "KB auto-sync failed: {}", e);
                        }
                        Err(e) => {
                            warn!(repo_id = %repo_id_clone, "KB auto-sync task panicked: {}", e);
                        }
                    }
                }
            }
        }
    });

    // The watcher thread and flush task run forever; if one dies we don't restart.
    // In practice the daemon restart cycle handles recovery.
    let _ = watcher_thread.join();
    recv_task.abort();
    flush_task.abort();
}
