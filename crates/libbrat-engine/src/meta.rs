//! Meta agent engine for the software factory.
//!
//! The Meta Agent is the evolved Mayor — a single orchestration surface that:
//! - Auto-bootstraps product and architecture KB notes on init
//! - Analyzes user intent and breaks it into phased pipeline plans
//! - Queries the knowledge base before creating convoys/tasks
//! - Enforces consistency gates between phases
//! - Writes memory notes after task completion
//!
//! The Meta Agent uses Claude Code's `--resume` flag to maintain conversation context.
//! State is persisted to `.brat/meta_state.json`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::engine::{
    Engine, EngineHealth, EngineInput, SessionHandle, SpawnResult, SpawnSpec, StopMode,
};
use crate::error::EngineError;

// Re-export bootstrap types
pub use crate::bootstrap::BootstrapResult;
pub use crate::consistency::{ConsistencyCheck, Inconsistency, InconsistencyKind, Severity};
pub use crate::infer_architecture::infer_architecture_notes;
pub use crate::infer_product::infer_product_notes;
pub use crate::scan::{scan_codebase, CodebaseScan, SourceFile};

/// Claude CLI JSON response format.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ClaudeResponse {
    #[serde(rename = "type")]
    response_type: String,
    result: Option<String>,
    session_id: String,
    is_error: Option<bool>,
}

/// Persisted meta agent state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaState {
    /// Claude session ID (persists conversation).
    pub session_id: String,
    /// Working directory.
    pub working_dir: PathBuf,
    /// Accumulated output lines from all calls.
    pub output_lines: Vec<String>,
    /// Whether the session is logically active.
    pub active: bool,
}

impl MetaState {
    /// Path to the state file within a repo.
    pub fn state_file_path(repo_root: &PathBuf) -> PathBuf {
        repo_root.join(".brat").join("meta_state.json")
    }

    /// Load state from disk.
    pub fn load(repo_root: &PathBuf) -> Option<Self> {
        let path = Self::state_file_path(repo_root);
        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str(&content) {
                    Ok(state) => Some(state),
                    Err(e) => {
                        warn!("failed to parse meta state: {}", e);
                        None
                    }
                },
                Err(e) => {
                    warn!("failed to read meta state: {}", e);
                    None
                }
            }
        } else {
            None
        }
    }

    /// Save state to disk.
    pub fn save(&self, repo_root: &PathBuf) -> Result<(), EngineError> {
        let path = Self::state_file_path(repo_root);
        let content = serde_json::to_string_pretty(self).map_err(|e| {
            EngineError::SpawnFailed(format!("failed to serialize meta state: {}", e))
        })?;
        fs::write(&path, content).map_err(|e| {
            EngineError::SpawnFailed(format!("failed to write meta state: {}", e))
        })?;
        Ok(())
    }

    /// Delete state file.
    pub fn delete(repo_root: &PathBuf) {
        let path = Self::state_file_path(repo_root);
        let _ = fs::remove_file(&path);
    }
}

/// Meta agent engine for software factory orchestration.
///
/// The Meta Agent manages the phased pipeline and knowledge base consistency.
pub struct MetaEngine {
    /// Repository root (for state persistence).
    repo_root: PathBuf,
}

impl MetaEngine {
    /// Create a new Meta engine for the given repo.
    pub fn new(repo_root: PathBuf) -> Self {
        Self { repo_root }
    }

    /// Check if a meta session is currently active.
    pub fn is_active(&self) -> bool {
        MetaState::load(&self.repo_root)
            .map(|s| s.active)
            .unwrap_or(false)
    }

    /// Get the current session ID if active.
    pub fn current_session_id(&self) -> Option<String> {
        MetaState::load(&self.repo_root)
            .filter(|s| s.active)
            .map(|s| s.session_id)
    }

    /// Get the current state if active.
    pub fn current_state(&self) -> Option<MetaState> {
        MetaState::load(&self.repo_root).filter(|s| s.active)
    }

    /// Auto-bootstrap the knowledge base for an existing repository.
    ///
    /// Scans the codebase, infers product and architecture notes, and checks
    /// consistency. Iterates up to `max_iterations` times trying to fix
    /// auto-fixable inconsistencies.
    pub async fn bootstrap(
        &self,
        repo_root: &Path,
        max_iterations: u32,
    ) -> Result<BootstrapResult, EngineError> {
        info!(repo_root = ?repo_root, "starting auto-bootstrap");

        let mut iterations = 0u32;
        let mut inconsistencies = Vec::new();
        let mut product_notes = Vec::new();
        let mut arch_notes = Vec::new();

        loop {
            // Step 1: Scan codebase
            let scan = scan_codebase(repo_root)
                .map_err(|e| EngineError::SpawnFailed(format!("scan failed: {}", e)))?;

            info!(
                files = scan.files.len(),
                docs = scan.docs.len(),
                "codebase scanned"
            );

            // Step 2: Infer product notes
            product_notes = infer_product_notes(&scan);
            info!(count = product_notes.len(), "inferred product notes");

            // Step 3: Infer architecture notes
            arch_notes = infer_architecture_notes(&scan, &product_notes);
            info!(count = arch_notes.len(), "inferred architecture notes");

            // Step 4: Consistency check
            let check = crate::consistency::check(&product_notes, &arch_notes, repo_root)
                .await
                .map_err(|e| EngineError::SpawnFailed(format!("consistency check failed: {}", e)))?;

            let score = check.score();
            inconsistencies = check.inconsistencies;
            info!(
                score,
                inconsistencies = inconsistencies.len(),
                "consistency check complete"
            );

            if inconsistencies.is_empty() || iterations >= max_iterations {
                break;
            }

            // Step 5: Try to auto-fix
            let fixed = crate::bootstrap::auto_fix(&mut inconsistencies);
            if !fixed {
                info!("no auto-fixes applied, surfacing to human");
                break;
            }

            iterations += 1;
        }

        let consistent = inconsistencies.is_empty();

        info!(
            consistent,
            iterations,
            "bootstrap complete"
        );

        Ok(BootstrapResult {
            consistent,
            iterations,
            inconsistencies,
            product_notes,
            arch_notes,
        })
    }

    /// Write meta agent context file to the workspace.
    fn write_meta_context(working_dir: &PathBuf, workflows: &[String]) -> Result<(), EngineError> {
        let context_dir = working_dir.join(".claude");
        fs::create_dir_all(&context_dir).map_err(|e| {
            EngineError::SpawnFailed(format!("failed to create .claude directory: {}", e))
        })?;

        let workflows_list = if workflows.is_empty() {
            "No workflows defined. You can create workflows in .brat/workflows/".to_string()
        } else {
            workflows
                .iter()
                .map(|w| format!("- {}", w))
                .collect::<Vec<_>>()
                .join("\n")
        };

        let context = format!(
            r#"# Meta Agent Context

You are the **Meta Agent** - the primary AI orchestrator for this software factory. Your role is to:

1. **Auto-bootstrap**: On init, scan the codebase and generate product/architecture KB notes until consistency
2. **Product Phase**: Analyze user intent and write structured product requirements to the KB
3. **Architecture Phase**: Read product requirements and write architecture design decisions to the KB
4. **Pipeline Orchestration**: Coordinate the phased pipeline: Product → Architecture → Implementation → Review → Merge
5. **Consistency Enforcement**: Ensure product and architecture notes are consistent before proceeding
6. **Memory Management**: After task completion, promote agent discoveries to permanent KB notes

## Your Capabilities

You have access to the `brat` CLI and knowledge base. Always use `--json` for machine-readable output.

### Knowledge Base Operations
```bash
# Search product notes
brat kb search --type product "query" --json

# Search architecture notes
brat kb search --type architecture "query" --json

# List inconsistencies
brat kb inconsistencies --json

# Get consistency score
brat kb score --json
```

### Convoy Management
```bash
# Create a new convoy (group of related tasks)
brat convoy create --title "Convoy Title" --body "Description" --json

# Check status
brat status --json
```

### Task Management
```bash
# Create a task within a convoy
brat task create --convoy <convoy_id> --title "Task Title" --body "Detailed instructions" --json

# Update task status
brat task update <task_id> --status <queued|running|blocked|needs-review|merged|dropped>
```

### Session Monitoring
```bash
# List active agent sessions
brat session list --json

# Show session details
brat session show <session_id> --json
```

## Available Workflows

{workflows_list}

## Guidelines

1. **KB First**: Always query the knowledge base before creating convoys or tasks
2. **Consistency Gates**: Never proceed to the next phase if consistency score < 100
3. **Human Escalation**: Surface inconsistencies and approval requests to humans, don't hide them
4. **Automated Memory**: After each task, write a memory note with discoveries
5. **TDD**: Architecture phase must define test strategy before implementation begins

## Important Notes

- Tasks are picked up by the Witness workflow and assigned to coding agents in swimlanes
- Each task runs in its own git worktree for isolation
- The Meta Agent is the single API/UI surface for the software factory
- Use convoy titles that describe the overall goal
- Use task titles that describe specific deliverables
"#,
            workflows_list = workflows_list
        );

        let context_file = context_dir.join("meta_context.md");
        fs::write(&context_file, &context).map_err(|e| {
            EngineError::SpawnFailed(format!("failed to write meta context: {}", e))
        })?;

        info!(context_file = ?context_file, "wrote meta context");
        Ok(())
    }

    /// Execute a Claude call with the given message and return the response.
    /// If session_id is provided, resumes that session; otherwise starts a new one.
    fn execute_claude_call(
        session_id: Option<&str>,
        working_dir: &PathBuf,
        message: &str,
    ) -> Result<(String, String), EngineError> {
        let escaped_message = message.replace("'", "'\\''");
        let shell_cmd = if let Some(sid) = session_id {
            format!(
                "claude --output-format json --print --permission-mode bypassPermissions --resume {} -p '{}'",
                sid, escaped_message
            )
        } else {
            format!(
                "claude --output-format json --print --permission-mode bypassPermissions -p '{}'",
                escaped_message
            )
        };

        info!(session_id = ?session_id, "executing claude call");
        debug!(shell_cmd = %shell_cmd, "claude command");

        let mut cmd = Command::new("bash");
        cmd.arg("-l").arg("-c").arg(&shell_cmd);
        cmd.current_dir(working_dir);
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output = cmd.output().map_err(|e| {
            EngineError::SpawnFailed(format!("failed to execute claude: {}", e))
        })?;

        if !output.stderr.is_empty() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            for line in stderr.lines() {
                warn!("claude stderr: {}", line);
            }
        }

        if !output.status.success() {
            warn!(exit_code = ?output.status.code(), "claude exited with error");
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let response: ClaudeResponse = serde_json::from_str(&stdout).map_err(|e| {
            EngineError::SpawnFailed(format!(
                "failed to parse claude response: {} (output: {})",
                e,
                stdout.chars().take(200).collect::<String>()
            ))
        })?;

        if response.is_error.unwrap_or(false) {
            return Err(EngineError::SpawnFailed(format!(
                "claude returned error: {}",
                response.result.unwrap_or_default()
            )));
        }

        let result_text = response.result.unwrap_or_default();
        Ok((response.session_id, result_text))
    }

    /// Send a message to the meta agent and return the response.
    pub fn ask(&self, message: &str) -> Result<Vec<String>, EngineError> {
        let mut state = MetaState::load(&self.repo_root).ok_or_else(|| {
            EngineError::SessionNotFound("no active meta session".to_string())
        })?;

        if !state.active {
            return Err(EngineError::SessionNotFound("meta session not active".to_string()));
        }

        let (new_session_id, result) = Self::execute_claude_call(
            Some(&state.session_id),
            &state.working_dir,
            message,
        )?;

        state.session_id = new_session_id;

        state.output_lines.push(format!(">>> {}", message));
        let response_lines: Vec<String> = result.lines().map(|s| s.to_string()).collect();
        state.output_lines.extend(response_lines.clone());
        state.output_lines.push(String::new());

        state.save(&self.repo_root)?;

        Ok(response_lines)
    }

    /// Get the last N lines of output.
    pub fn tail(&self, n: usize) -> Result<Vec<String>, EngineError> {
        let state = MetaState::load(&self.repo_root).ok_or_else(|| {
            EngineError::SessionNotFound("no active meta session".to_string())
        })?;

        let lines = &state.output_lines;
        let start = lines.len().saturating_sub(n);
        Ok(lines[start..].to_vec())
    }

    /// Stop the meta session.
    pub fn stop_session(&self) -> Result<(), EngineError> {
        if !self.is_active() {
            return Err(EngineError::SessionNotFound("no active meta session".to_string()));
        }

        MetaState::delete(&self.repo_root);
        info!("meta session stopped");
        Ok(())
    }
}

impl Default for MetaEngine {
    fn default() -> Self {
        Self::new(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }
}

#[async_trait]
impl Engine for MetaEngine {
    async fn spawn(&self, spec: SpawnSpec) -> Result<SpawnResult, EngineError> {
        if self.is_active() {
            return Err(EngineError::SpawnFailed(
                "meta session already active - stop it first".to_string(),
            ));
        }

        info!(working_dir = ?spec.working_dir, "starting meta session");

        let workflows_dir = spec.working_dir.join(".brat/workflows");
        let workflows: Vec<String> = if workflows_dir.exists() {
            fs::read_dir(&workflows_dir)
                .map(|entries| {
                    entries
                        .filter_map(|e| e.ok())
                        .filter(|e| {
                            e.path()
                                .extension()
                                .map(|ext| ext == "yaml" || ext == "yml")
                                .unwrap_or(false)
                        })
                        .filter_map(|e| {
                            e.path()
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .map(|s| s.to_string())
                        })
                        .collect()
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        Self::write_meta_context(&spec.working_dir, &workflows)?;

        let initial_message = if spec.command.is_empty() {
            "You are the Meta Agent. Read your context from .claude/meta_context.md and confirm you understand your role. Briefly list your main capabilities.".to_string()
        } else {
            spec.command.clone()
        };

        let (session_id, result) = Self::execute_claude_call(
            None,
            &spec.working_dir,
            &initial_message,
        )?;

        let mut output_lines = vec![format!(">>> {}", initial_message)];
        output_lines.extend(result.lines().map(|s| s.to_string()));
        output_lines.push(String::new());

        let state = MetaState {
            session_id: session_id.clone(),
            working_dir: spec.working_dir.clone(),
            output_lines,
            active: true,
        };

        state.save(&self.repo_root)?;

        info!(session_id = %session_id, "meta session started");

        Ok(SpawnResult {
            session_id,
            pid: std::process::id(),
        })
    }

    async fn send(&self, _session: &SessionHandle, input: EngineInput) -> Result<(), EngineError> {
        match input {
            EngineInput::Text(text) => {
                self.ask(&text)?;
                Ok(())
            }
            EngineInput::Signal(_) => {
                Err(EngineError::SendFailed("signals not supported for meta".to_string()))
            }
        }
    }

    async fn tail(&self, _session: &SessionHandle, n: usize) -> Result<Vec<String>, EngineError> {
        self.tail(n)
    }

    async fn stop(&self, _session: &SessionHandle, _how: StopMode) -> Result<(), EngineError> {
        self.stop_session()
    }

    async fn health(&self, _session: &SessionHandle) -> Result<EngineHealth, EngineError> {
        if self.is_active() {
            Ok(EngineHealth::alive(std::process::id()))
        } else {
            Ok(EngineHealth::exited(0, "session not active".to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_meta_engine_creation() {
        let dir = tempdir().unwrap();
        let engine = MetaEngine::new(dir.path().to_path_buf());
        assert!(!engine.is_active());
        assert!(engine.current_session_id().is_none());
    }

    #[test]
    fn test_meta_state_persistence() {
        let dir = tempdir().unwrap();
        let repo_root = dir.path().to_path_buf();

        fs::create_dir_all(repo_root.join(".brat")).unwrap();

        let state = MetaState {
            session_id: "test-123".to_string(),
            working_dir: repo_root.clone(),
            output_lines: vec!["line1".to_string(), "line2".to_string()],
            active: true,
        };

        state.save(&repo_root).unwrap();
        let loaded = MetaState::load(&repo_root).unwrap();

        assert_eq!(loaded.session_id, "test-123");
        assert_eq!(loaded.output_lines.len(), 2);
        assert!(loaded.active);

        MetaState::delete(&repo_root);
        assert!(MetaState::load(&repo_root).is_none());
    }
}
