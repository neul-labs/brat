use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// Brat - Multi-agent coding harness backed by Grite
#[derive(Parser, Debug)]
#[command(name = "brat", version, about, long_about = None)]
pub struct Cli {
    /// Output in JSON format
    #[arg(long, global = true)]
    pub json: bool,

    /// Suppress human-readable output
    #[arg(long, global = true)]
    pub quiet: bool,

    /// Target a specific repository
    #[arg(long, global = true)]
    pub repo: Option<PathBuf>,

    /// Don't auto-start the daemon (run in standalone mode)
    #[arg(long, global = true)]
    pub no_daemon: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Initialize Brat in the current repository
    Init(InitArgs),

    /// Show harness status
    Status(StatusArgs),

    /// Convoy management
    #[command(subcommand)]
    Convoy(ConvoyCommand),

    /// Task management
    #[command(subcommand)]
    Task(TaskCommand),

    /// Context store (symbol extraction and querying)
    #[command(subcommand)]
    Context(ContextCommand),

    /// Witness workflow (polecat session management)
    #[command(subcommand)]
    Witness(WitnessCommand),

    /// Refinery workflow (merge queue management)
    #[command(subcommand)]
    Refinery(RefineryCommand),

    /// Session management
    #[command(subcommand)]
    Session(SessionCommand),

    /// Lock status and management
    #[command(subcommand)]
    Lock(LockCommand),

    /// Health check and diagnostics
    Doctor(DoctorArgs),

    /// Start the HTTP API server (bratd daemon)
    Api(ApiArgs),

    /// Workflow template management
    #[command(subcommand)]
    Workflow(WorkflowCommand),

    /// AI-driven Meta orchestrator
    #[command(subcommand)]
    Meta(MetaCommand),

    /// Daemon management (start/stop/status)
    #[command(subcommand)]
    Daemon(DaemonCommand),

    /// Knowledge base operations
    #[command(subcommand)]
    Kb(KbCommand),

    /// Bootstrap operations
    #[command(subcommand)]
    Bootstrap(BootstrapCommand),

    /// Skill management
    #[command(subcommand)]
    Skill(SkillCommand),

    /// MCP server
    Mcp(McpArgs),
}

/// Arguments for the init command
#[derive(Parser, Debug)]
pub struct InitArgs {
    /// Don't start the bratd daemon
    #[arg(long)]
    pub no_daemon: bool,

    /// Don't create the tmux control room
    #[arg(long)]
    pub no_tmux: bool,

    /// Don't create .brat/config.toml
    #[arg(long)]
    pub no_config: bool,

    /// Don't create/update AGENTS.md
    #[arg(long)]
    pub no_agents_md: bool,

    /// Don't install git hooks
    #[arg(long)]
    pub no_hooks: bool,
}

/// Arguments for the status command
#[derive(Parser, Debug)]
pub struct StatusArgs {
    /// Aggregate status across all configured repos
    #[arg(long)]
    pub all_repos: bool,

    /// Filter by convoy ID
    #[arg(long)]
    pub convoy: Option<String>,

    /// Watch for changes (streaming mode)
    #[arg(long)]
    pub watch: bool,

    /// Poll interval in seconds for watch mode
    #[arg(long, default_value = "2")]
    pub poll_interval: u64,
}

/// Convoy subcommands
#[derive(Subcommand, Debug)]
pub enum ConvoyCommand {
    /// Create a new convoy
    Create(ConvoyCreateArgs),
}

/// Arguments for convoy create
#[derive(Parser, Debug)]
pub struct ConvoyCreateArgs {
    /// Convoy title
    #[arg(long)]
    pub title: String,

    /// Convoy body/description
    #[arg(long)]
    pub body: Option<String>,
}

/// Task subcommands
#[derive(Subcommand, Debug)]
pub enum TaskCommand {
    /// Create a new task
    Create(TaskCreateArgs),

    /// Update task status
    Update(TaskUpdateArgs),

    /// Manage task dependencies
    #[command(subcommand)]
    Dep(TaskDepCommand),
}

/// Task dependency subcommands
#[derive(Subcommand, Debug)]
pub enum TaskDepCommand {
    /// Add a dependency between tasks
    Add(TaskDepAddArgs),

    /// Remove a dependency between tasks
    Remove(TaskDepRemoveArgs),

    /// List dependencies for a task
    List(TaskDepListArgs),

    /// Show tasks in topological order (ready-to-run first)
    Topo(TaskDepTopoArgs),
}

/// Arguments for task create
#[derive(Parser, Debug)]
pub struct TaskCreateArgs {
    /// Convoy ID to link the task to
    #[arg(long)]
    pub convoy: String,

    /// Task title
    #[arg(long)]
    pub title: String,

    /// Task body/description
    #[arg(long)]
    pub body: Option<String>,
}

/// Arguments for task update
#[derive(Parser, Debug)]
pub struct TaskUpdateArgs {
    /// Task ID to update
    pub task_id: String,

    /// New status (queued, running, blocked, needs-review, merged, dropped)
    #[arg(long)]
    pub status: String,

    /// Force the transition (bypass state machine validation)
    #[arg(long)]
    pub force: bool,
}

/// Arguments for task dep add
#[derive(Parser, Debug)]
pub struct TaskDepAddArgs {
    /// Task ID (source of the dependency)
    pub task_id: String,

    /// Target task ID (the task this one depends on or blocks)
    #[arg(long)]
    pub target: String,

    /// Dependency type: blocks, depends_on, or related_to
    #[arg(long, short = 't', default_value = "depends_on")]
    pub dep_type: String,
}

/// Arguments for task dep remove
#[derive(Parser, Debug)]
pub struct TaskDepRemoveArgs {
    /// Task ID (source of the dependency)
    pub task_id: String,

    /// Target task ID
    #[arg(long)]
    pub target: String,

    /// Dependency type: blocks, depends_on, or related_to
    #[arg(long, short = 't', default_value = "depends_on")]
    pub dep_type: String,
}

/// Arguments for task dep list
#[derive(Parser, Debug)]
pub struct TaskDepListArgs {
    /// Task ID to list dependencies for
    pub task_id: String,

    /// List dependents instead of dependencies (reverse direction)
    #[arg(long, short = 'r')]
    pub reverse: bool,
}

/// Arguments for task dep topo
#[derive(Parser, Debug)]
pub struct TaskDepTopoArgs {
    /// Filter by convoy ID
    #[arg(long)]
    pub convoy: Option<String>,
}

/// Context subcommands
#[derive(Subcommand, Debug)]
pub enum ContextCommand {
    /// Index files for symbol extraction
    Index(ContextIndexArgs),

    /// Query for symbols
    Query(ContextQueryArgs),

    /// Show context for a file
    Show(ContextShowArgs),

    /// Get or list project context
    Project(ContextProjectArgs),

    /// Set a project context value
    Set(ContextSetArgs),
}

/// Arguments for context index
#[derive(Parser, Debug)]
pub struct ContextIndexArgs {
    /// Paths to index (if empty, indexes all tracked files)
    #[arg(long)]
    pub path: Vec<String>,

    /// Force re-indexing even if content hasn't changed
    #[arg(long)]
    pub force: bool,

    /// Glob pattern to filter files
    #[arg(long)]
    pub pattern: Option<String>,
}

/// Arguments for context query
#[derive(Parser, Debug)]
pub struct ContextQueryArgs {
    /// Symbol pattern to search for
    pub query: String,
}

/// Arguments for context show
#[derive(Parser, Debug)]
pub struct ContextShowArgs {
    /// File path to show context for
    pub path: String,
}

/// Arguments for context project
#[derive(Parser, Debug)]
pub struct ContextProjectArgs {
    /// Key to get (if omitted, lists all entries)
    pub key: Option<String>,
}

/// Arguments for context set
#[derive(Parser, Debug)]
pub struct ContextSetArgs {
    /// Key to set
    pub key: String,

    /// Value to set
    pub value: String,
}

/// Witness subcommands
#[derive(Subcommand, Debug)]
pub enum WitnessCommand {
    /// Run the witness workflow
    Run(WitnessRunArgs),
}

/// Arguments for witness run
#[derive(Parser, Debug)]
pub struct WitnessRunArgs {
    /// Run once and exit (default: run as daemon)
    #[arg(long)]
    pub once: bool,

    /// Poll interval in seconds for daemon mode
    #[arg(long, default_value = "10")]
    pub poll_interval: u64,

    /// Skip session reconciliation on startup
    #[arg(long)]
    pub skip_reconcile: bool,

    /// Engine to use for spawning sessions. Overrides config.
    /// Options: claude-code, codex, opencode, aider, gemini, copilot, continue, shell
    #[arg(long, short = 'e')]
    pub engine: Option<String>,
}

/// Refinery subcommands
#[derive(Subcommand, Debug)]
pub enum RefineryCommand {
    /// Run the refinery workflow
    Run(RefineryRunArgs),
}

/// Arguments for refinery run
#[derive(Parser, Debug)]
pub struct RefineryRunArgs {
    /// Run once and exit (default: run as daemon)
    #[arg(long)]
    pub once: bool,

    /// Poll interval in seconds for daemon mode
    #[arg(long, default_value = "10")]
    pub poll_interval: u64,
}

/// Session subcommands
#[derive(Subcommand, Debug)]
pub enum SessionCommand {
    /// List active sessions
    List(SessionListArgs),
    /// Show session details
    Show(SessionShowArgs),
    /// Stop a session gracefully
    Stop(SessionStopArgs),
    /// Tail session logs
    Tail(SessionTailArgs),
}

/// Arguments for session list
#[derive(Parser, Debug)]
pub struct SessionListArgs {
    /// Filter by task ID
    #[arg(long)]
    pub task: Option<String>,
}

/// Arguments for session show
#[derive(Parser, Debug)]
pub struct SessionShowArgs {
    /// Session ID to show
    pub session_id: String,
}

/// Arguments for session stop
#[derive(Parser, Debug)]
pub struct SessionStopArgs {
    /// Session ID to stop
    pub session_id: String,

    /// Reason for stopping
    #[arg(long, default_value = "user-stop")]
    pub reason: String,
}

/// Arguments for session tail
#[derive(Parser, Debug)]
pub struct SessionTailArgs {
    /// Session ID to tail
    pub session_id: String,

    /// Number of lines to show
    #[arg(long, short = 'n', default_value = "50")]
    pub lines: usize,

    /// Follow log output (stream new lines)
    #[arg(long, short = 'f')]
    pub follow: bool,
}

/// Lock subcommands
#[derive(Subcommand, Debug)]
pub enum LockCommand {
    /// Show lock status
    Status(LockStatusArgs),
}

/// Arguments for lock status
#[derive(Parser, Debug)]
pub struct LockStatusArgs {
    /// Show only conflicting locks
    #[arg(long)]
    pub conflicts_only: bool,
}

/// Arguments for doctor command
#[derive(Parser, Debug)]
pub struct DoctorArgs {
    /// Check mode (read-only health validation)
    #[arg(long, conflicts_with = "rebuild")]
    pub check: bool,

    /// Rebuild mode (rebuilds harness state)
    #[arg(long, conflicts_with = "check")]
    pub rebuild: bool,
}

/// Arguments for API server command
#[derive(Parser, Debug)]
pub struct ApiArgs {
    /// Host to bind to
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,

    /// Port to listen on
    #[arg(long, short = 'p', default_value = "3000")]
    pub port: u16,

    /// CORS allowed origin (default: allow all)
    #[arg(long)]
    pub cors_origin: Option<String>,

    /// Idle timeout in seconds. Daemon shuts down after this period of inactivity.
    /// Set to 0 to disable idle shutdown.
    #[arg(long, default_value = "900")]
    pub idle_timeout: u64,
}

/// Workflow subcommands
#[derive(Subcommand, Debug)]
pub enum WorkflowCommand {
    /// List available workflows
    List(WorkflowListArgs),

    /// Show workflow details
    Show(WorkflowShowArgs),

    /// Run a workflow
    Run(WorkflowRunArgs),
}

/// Arguments for workflow list
#[derive(Parser, Debug)]
pub struct WorkflowListArgs {
    // No additional arguments needed
}

/// Arguments for workflow show
#[derive(Parser, Debug)]
pub struct WorkflowShowArgs {
    /// Workflow name to show
    pub name: String,
}

/// Arguments for workflow run
#[derive(Parser, Debug)]
pub struct WorkflowRunArgs {
    /// Workflow name to run
    pub name: String,

    /// Variable assignments (key=value)
    #[arg(long = "var", short = 'v', value_parser = parse_var)]
    pub vars: Vec<(String, String)>,
}

/// Parse a key=value variable assignment.
fn parse_var(s: &str) -> Result<(String, String), String> {
    let parts: Vec<&str> = s.splitn(2, '=').collect();
    if parts.len() != 2 {
        return Err(format!("invalid variable format '{}', expected key=value", s));
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

/// Meta subcommands
#[derive(Subcommand, Debug)]
pub enum MetaCommand {
    /// Start the Meta orchestrator
    Start(MetaStartArgs),

    /// Send a message to the Meta Agent
    Ask(MetaAskArgs),

    /// Check Meta status
    Status(MetaStatusArgs),

    /// View Meta output
    Tail(MetaTailArgs),

    /// Stop the Meta Agent
    Stop(MetaStopArgs),
}

/// Arguments for meta start
#[derive(Parser, Debug)]
pub struct MetaStartArgs {
    /// Initial message/instruction for the Meta Agent
    #[arg(long, short = 'm')]
    pub message: Option<String>,
}

/// Arguments for meta ask
#[derive(Parser, Debug)]
pub struct MetaAskArgs {
    /// Message to send to the Meta Agent
    pub message: String,
}

/// Arguments for meta status
#[derive(Parser, Debug)]
pub struct MetaStatusArgs {
    // No additional arguments needed
}

/// Arguments for meta tail
#[derive(Parser, Debug)]
pub struct MetaTailArgs {
    /// Number of lines to show
    #[arg(long, short = 'n', default_value = "50")]
    pub lines: usize,
}

/// Arguments for meta stop
#[derive(Parser, Debug)]
pub struct MetaStopArgs {
    /// Force kill instead of graceful stop
    #[arg(long)]
    pub force: bool,
}

/// Knowledge base subcommands
#[derive(Subcommand, Debug)]
pub enum KbCommand {
    /// Search the knowledge base
    Search(KbSearchArgs),
    /// List product notes
    Product(KbProductArgs),
    /// List architecture notes
    Architecture(KbArchitectureArgs),
    /// Show consistency score
    Score(KbScoreArgs),
    /// List inconsistencies
    Inconsistencies(KbInconsistenciesArgs),
    /// Sync from filesystem and run consistency check
    Check(KbCheckArgs),
    /// Open a note in $EDITOR and sync after save
    Edit(KbEditArgs),
}

/// Arguments for kb search
#[derive(Parser, Debug)]
pub struct KbSearchArgs {
    /// Search query
    pub query: String,
    /// Filter by note type
    #[arg(long)]
    pub note_type: Option<String>,
}

/// Arguments for kb product
#[derive(Parser, Debug)]
pub struct KbProductArgs {
    // No additional arguments needed
}

/// Arguments for kb architecture
#[derive(Parser, Debug)]
pub struct KbArchitectureArgs {
    // No additional arguments needed
}

/// Arguments for kb score
#[derive(Parser, Debug)]
pub struct KbScoreArgs {
    // No additional arguments needed
}

/// Arguments for kb inconsistencies
#[derive(Parser, Debug)]
pub struct KbInconsistenciesArgs {
    // No additional arguments needed
}

/// Arguments for kb check
#[derive(Parser, Debug)]
pub struct KbCheckArgs {
    /// Minimum consistency score required (0-100). Fails with exit code 1 if not met.
    #[arg(long)]
    pub min_score: Option<u8>,
}

/// Arguments for kb edit
#[derive(Parser, Debug)]
pub struct KbEditArgs {
    /// Note slug to edit
    pub slug: String,

    /// Skip consistency check after editing
    #[arg(long)]
    pub no_check: bool,
}

/// Bootstrap subcommands
#[derive(Subcommand, Debug)]
pub enum BootstrapCommand {
    /// Run bootstrap on existing repo
    Run(BootstrapRunArgs),
}

/// Arguments for bootstrap run
#[derive(Parser, Debug)]
pub struct BootstrapRunArgs {
    /// Max iterations for consistency fix attempts
    #[arg(long, default_value = "5")]
    pub max_iterations: u32,
}

/// Skill subcommands
#[derive(Subcommand, Debug)]
pub enum SkillCommand {
    /// Install all brat skills
    Install(SkillInstallArgs),
    /// List embedded skills
    List(SkillListArgs),
}

/// Arguments for skill install
#[derive(Parser, Debug)]
pub struct SkillInstallArgs {
    /// Force reinstall even if already installed
    #[arg(long)]
    pub force: bool,
}

/// Arguments for skill list
#[derive(Parser, Debug)]
pub struct SkillListArgs {
    // No additional arguments needed
}

/// Arguments for MCP server
#[derive(Parser, Debug)]
pub struct McpArgs {
    // No additional arguments needed
}

/// Daemon subcommands
#[derive(Subcommand, Debug)]
pub enum DaemonCommand {
    /// Start the daemon in background
    Start(DaemonStartArgs),

    /// Stop the daemon
    Stop(DaemonStopArgs),

    /// Show daemon status
    Status(DaemonStatusArgs),

    /// Restart the daemon
    Restart(DaemonRestartArgs),

    /// Show daemon logs
    Logs(DaemonLogsArgs),
}

/// Arguments for daemon start
#[derive(Parser, Debug)]
pub struct DaemonStartArgs {
    /// Port to listen on
    #[arg(long, short = 'p', default_value = "3000")]
    pub port: u16,

    /// Idle timeout in seconds (0 = no timeout)
    #[arg(long, default_value = "900")]
    pub idle_timeout: u64,

    /// Run in foreground (don't daemonize)
    #[arg(long)]
    pub foreground: bool,
}

/// Arguments for daemon stop
#[derive(Parser, Debug)]
pub struct DaemonStopArgs {
    /// Force kill instead of graceful stop
    #[arg(long)]
    pub force: bool,
}

/// Arguments for daemon status
#[derive(Parser, Debug)]
pub struct DaemonStatusArgs {
    // No additional arguments needed
}

/// Arguments for daemon restart
#[derive(Parser, Debug)]
pub struct DaemonRestartArgs {
    /// Port to listen on
    #[arg(long, short = 'p', default_value = "3000")]
    pub port: u16,

    /// Idle timeout in seconds (0 = no timeout)
    #[arg(long, default_value = "900")]
    pub idle_timeout: u64,
}

/// Arguments for daemon logs
#[derive(Parser, Debug)]
pub struct DaemonLogsArgs {
    /// Number of lines to show
    #[arg(long, short = 'n', default_value = "50")]
    pub lines: usize,
}
