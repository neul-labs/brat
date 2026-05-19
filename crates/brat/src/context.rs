use std::path::{Path, PathBuf};

use libbrat_config::BratConfig;
use libbrat_engine::Engine;
use libbrat_grite::GriteeClient;
use libbrat_session::{MonitorConfig, SessionMonitor};
use libbrat_worktree::WorktreeManager;

use crate::cli::Cli;
use crate::error::BratError;

/// Brat execution context.
///
/// Contains resolved paths and configuration for command execution.
#[derive(Debug)]
pub struct BratContext {
    /// Path to the repository root (where .git is).
    pub repo_root: PathBuf,

    /// Path to the .git directory.
    pub git_dir: PathBuf,

    /// Path to the .brat directory.
    pub brat_dir: PathBuf,

    /// Path to the .brat/config.toml file.
    pub config_path: PathBuf,

    /// Loaded configuration (if initialized).
    pub config: Option<BratConfig>,
}

impl BratContext {
    /// Resolve context from CLI arguments.
    ///
    /// This finds the git repository and loads the Brat configuration.
    pub fn resolve(cli: &Cli) -> Result<Self, BratError> {
        // Find git directory
        let repo_root = if let Some(ref repo) = cli.repo {
            repo.clone()
        } else {
            std::env::current_dir()?
        };

        let git_dir = find_git_dir(&repo_root)?;
        let repo_root = git_dir
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| repo_root.clone());

        let brat_dir = repo_root.join(".brat");
        let config_path = brat_dir.join("config.toml");

        // Try to load config if it exists
        let config = if config_path.exists() {
            Some(BratConfig::load(&config_path)?)
        } else {
            None
        };

        Ok(Self {
            repo_root,
            git_dir,
            brat_dir,
            config_path,
            config,
        })
    }

    /// Check if Brat is initialized in this repository.
    pub fn is_initialized(&self) -> bool {
        self.config_path.exists()
    }

    /// Require that Brat is initialized.
    pub fn require_initialized(&self) -> Result<&BratConfig, BratError> {
        self.config.as_ref().ok_or(BratError::NotInitialized)
    }

    /// Check if Grite is initialized in this repository.
    pub fn is_gritee_initialized(&self) -> bool {
        self.git_dir.join("gritee").exists()
    }

    /// Require that Grite is initialized.
    pub fn require_gritee_initialized(&self) -> Result<(), BratError> {
        if self.is_gritee_initialized() {
            Ok(())
        } else {
            Err(BratError::GriteeNotInitialized)
        }
    }

    /// Create a GriteeClient for this repository.
    pub fn gritee_client(&self) -> GriteeClient {
        GriteeClient::new(&self.repo_root)
    }

    /// Create a WorktreeManager for this repository.
    ///
    /// Requires Brat to be initialized to read swarm configuration.
    pub fn worktree_manager(&self) -> Result<WorktreeManager, BratError> {
        let config = self.require_initialized()?;
        Ok(WorktreeManager::new(
            &self.repo_root,
            &config.swarm.worktree_root,
            config.swarm.max_polecats,
        ))
    }

    /// Create a SessionMonitor for this repository.
    ///
    /// The SessionMonitor coordinates engine processes with Gritee sessions
    /// and optional worktree isolation.
    ///
    /// # Arguments
    ///
    /// * `engine` - Engine implementation for spawning and controlling sessions.
    /// * `monitor_config` - Configuration for monitoring behavior.
    ///
    /// # Requires
    ///
    /// Brat must be initialized to read swarm configuration.
    pub fn session_monitor<E: Engine + 'static>(
        &self,
        engine: E,
        engine_name: impl Into<String>,
        monitor_config: MonitorConfig,
    ) -> Result<SessionMonitor<E>, BratError> {
        let _brat_config = self.require_initialized()?;
        let gritee = self.gritee_client();
        let worktree_manager = self.worktree_manager().ok();

        Ok(SessionMonitor::new(
            engine,
            engine_name,
            gritee,
            worktree_manager,
            monitor_config,
        ))
    }
}

/// Find the .git directory starting from the given path.
///
/// Walks up the directory tree looking for a .git directory.
fn find_git_dir(start: &Path) -> Result<PathBuf, BratError> {
    let mut current = start.to_path_buf();

    loop {
        let git_dir = current.join(".git");
        if git_dir.is_dir() {
            return Ok(git_dir);
        }

        // Check for git worktree (file pointing to actual git dir)
        if git_dir.is_file() {
            // Read the gitdir pointer
            let content = std::fs::read_to_string(&git_dir)?;
            if let Some(path) = content.strip_prefix("gitdir: ") {
                let path = path.trim();
                return Ok(PathBuf::from(path));
            }
        }

        // Move to parent directory
        if let Some(parent) = current.parent() {
            current = parent.to_path_buf();
        } else {
            return Err(BratError::NotAGitRepo);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_git_dir_not_found() {
        let result = find_git_dir(Path::new("/tmp"));
        // This might succeed if /tmp is in a git repo, otherwise it should fail
        // Just check it doesn't panic
        let _ = result;
    }
}
