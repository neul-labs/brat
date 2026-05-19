use serde::{Deserialize, Serialize};
use std::path::Path;

/// Brat harness configuration.
///
/// This is stored in `.brat/config.toml` and controls harness behavior.
/// Grite configuration remains separate in `.git/gritee/config.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BratConfig {
    /// Role enablement settings.
    pub roles: RolesConfig,

    /// Daemon settings.
    pub bratd: BratdConfig,

    /// Swarm settings.
    pub swarm: SwarmConfig,

    /// Engine timeout settings.
    pub engine: EngineConfig,

    /// Refinery (merge) settings.
    pub refinery: RefineryConfig,

    /// Lock policy settings.
    pub locks: LocksConfig,

    /// Tmux control room settings.
    pub tmux: TmuxConfig,

    /// Multi-repo settings.
    pub repos: ReposConfig,

    /// Log retention settings.
    pub logs: LogsConfig,

    /// Knowledge base settings.
    pub kb: KbConfig,

    /// Swimlanes settings.
    pub swimlanes: SwimlanesConfig,

    /// Bootstrap settings.
    pub bootstrap: BootstrapConfig,

    /// Intervention thresholds.
    pub interventions: InterventionsConfig,
}

impl Default for BratConfig {
    fn default() -> Self {
        Self {
            roles: RolesConfig::default(),
            bratd: BratdConfig::default(),
            swarm: SwarmConfig::default(),
            engine: EngineConfig::default(),
            refinery: RefineryConfig::default(),
            locks: LocksConfig::default(),
            tmux: TmuxConfig::default(),
            repos: ReposConfig::default(),
            logs: LogsConfig::default(),
            interventions: InterventionsConfig::default(),
            kb: KbConfig::default(),
            swimlanes: SwimlanesConfig::default(),
            bootstrap: BootstrapConfig::default(),
        }
    }
}

/// Role enablement settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RolesConfig {
    pub mayor_enabled: bool,
    pub witness_enabled: bool,
    pub refinery_enabled: bool,
    pub deacon_enabled: bool,
}

impl Default for RolesConfig {
    fn default() -> Self {
        Self {
            mayor_enabled: true,
            witness_enabled: true,
            refinery_enabled: true,
            deacon_enabled: true,
        }
    }
}

/// Daemon settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BratdConfig {
    pub enabled: bool,
    pub start_gritee_daemon: bool,
}

impl Default for BratdConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            start_gritee_daemon: false,
        }
    }
}

/// Swarm settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SwarmConfig {
    pub max_polecats: u32,
    pub worktree_root: String,
    pub engine: String,
}

impl Default for SwarmConfig {
    fn default() -> Self {
        Self {
            max_polecats: 6,
            worktree_root: ".gritee/worktrees".to_string(),
            engine: "codex".to_string(),
        }
    }
}

/// Engine timeout settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EngineConfig {
    pub spawn_timeout_ms: u64,
    pub send_timeout_ms: u64,
    pub tail_timeout_ms: u64,
    pub stop_timeout_ms: u64,
    pub health_timeout_ms: u64,
    pub spawn_retry: u32,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            spawn_timeout_ms: 60_000,
            send_timeout_ms: 5_000,
            tail_timeout_ms: 10_000,
            stop_timeout_ms: 10_000,
            health_timeout_ms: 5_000,
            spawn_retry: 1,
        }
    }
}

/// Refinery (merge) settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RefineryConfig {
    pub max_parallel_merges: u32,
    pub rebase_strategy: String,
    pub required_checks: Vec<String>,
    pub merge_retry_limit: u32,
}

impl Default for RefineryConfig {
    fn default() -> Self {
        Self {
            max_parallel_merges: 2,
            rebase_strategy: "rebase".to_string(),
            required_checks: vec!["tests".to_string()],
            merge_retry_limit: 2,
        }
    }
}

/// Lock policy settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LocksConfig {
    /// Lock policy: "off", "warn", or "require"
    pub policy: String,
}

impl Default for LocksConfig {
    fn default() -> Self {
        Self {
            policy: "warn".to_string(),
        }
    }
}

/// Tmux control room settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TmuxConfig {
    pub session: String,
    pub windows: Vec<String>,
}

impl Default for TmuxConfig {
    fn default() -> Self {
        Self {
            session: "brat".to_string(),
            windows: vec![
                "mayor".to_string(),
                "witness".to_string(),
                "refinery".to_string(),
                "deacon".to_string(),
                "sessions".to_string(),
            ],
        }
    }
}

/// Multi-repo settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ReposConfig {
    pub roots: Vec<String>,
}

impl Default for ReposConfig {
    fn default() -> Self {
        Self { roots: Vec::new() }
    }
}

/// Log retention settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LogsConfig {
    pub retention_days: u32,
}

impl Default for LogsConfig {
    fn default() -> Self {
        Self { retention_days: 7 }
    }
}

/// Intervention thresholds.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct InterventionsConfig {
    pub heartbeat_interval_ms: u64,
    pub stale_session_ms: u64,
    pub blocked_task_ms: u64,
}

impl Default for InterventionsConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval_ms: 30_000,
            stale_session_ms: 300_000,       // 5 minutes
            blocked_task_ms: 86_400_000,     // 24 hours
        }
    }
}

/// Knowledge base settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct KbConfig {
    pub enabled: bool,
    pub tenant_prefix: String,
    pub mirror_path: String,
    pub min_consistency_score: u8,
}

impl Default for KbConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            tenant_prefix: "brat-".to_string(),
            mirror_path: ".brat/kb".to_string(),
            min_consistency_score: 80,
        }
    }
}

/// Swimlanes settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SwimlanesConfig {
    pub enabled: bool,
    pub max_count: u32,
    pub teams: Vec<String>,
    pub engines: std::collections::HashMap<String, String>,
}

impl Default for SwimlanesConfig {
    fn default() -> Self {
        let mut engines = std::collections::HashMap::new();
        engines.insert("backend".to_string(), "codex".to_string());
        engines.insert("frontend".to_string(), "claude".to_string());
        engines.insert("tests".to_string(), "codex".to_string());
        engines.insert("docs".to_string(), "claude".to_string());

        Self {
            enabled: true,
            max_count: 4,
            teams: vec![
                "backend".to_string(),
                "frontend".to_string(),
                "tests".to_string(),
                "docs".to_string(),
            ],
            engines,
        }
    }
}

/// Bootstrap settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BootstrapConfig {
    pub enabled: bool,
    pub max_iterations: u32,
    pub max_files: u32,
    pub timeout_secs: u64,
    pub require_human_approval: bool,
}

impl Default for BootstrapConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_iterations: 5,
            max_files: 10_000,
            timeout_secs: 300,
            require_human_approval: true,
        }
    }
}

/// Configuration error.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read config: {0}")]
    ReadError(#[from] std::io::Error),

    #[error("failed to parse config: {0}")]
    ParseError(#[from] toml::de::Error),

    #[error("failed to serialize config: {0}")]
    SerializeError(#[from] toml::ser::Error),

    #[error("invalid config: {0}")]
    ValidationError(String),
}

impl BratConfig {
    /// Load config from a file path.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        let config: BratConfig = toml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    /// Save config to a file path.
    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        let content = toml::to_string_pretty(self)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Validate the configuration.
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate lock policy
        match self.locks.policy.as_str() {
            "off" | "warn" | "require" => {}
            other => {
                return Err(ConfigError::ValidationError(format!(
                    "invalid lock policy '{}', must be 'off', 'warn', or 'require'",
                    other
                )));
            }
        }

        // Validate rebase strategy
        match self.refinery.rebase_strategy.as_str() {
            "rebase" | "merge" | "squash" => {}
            other => {
                return Err(ConfigError::ValidationError(format!(
                    "invalid rebase strategy '{}', must be 'rebase', 'merge', or 'squash'",
                    other
                )));
            }
        }

        // Validate bootstrap config
        if self.bootstrap.max_iterations == 0 {
            return Err(ConfigError::ValidationError(
                "bootstrap.max_iterations must be > 0".to_string(),
            ));
        }

        // Validate swimlanes config
        if self.swimlanes.enabled && self.swimlanes.teams.is_empty() {
            return Err(ConfigError::ValidationError(
                "swimlanes.teams cannot be empty when enabled".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = BratConfig::default();
        assert!(config.roles.mayor_enabled);
        assert_eq!(config.swarm.max_polecats, 6);
        assert_eq!(config.locks.policy, "warn");
    }

    #[test]
    fn test_config_serialization() {
        let config = BratConfig::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(toml_str.contains("[roles]"));
        assert!(toml_str.contains("mayor_enabled = true"));
    }

    #[test]
    fn test_config_validation() {
        let mut config = BratConfig::default();
        assert!(config.validate().is_ok());

        config.locks.policy = "invalid".to_string();
        assert!(config.validate().is_err());
    }
}
