//! Session command handler.

use std::process::Command;
use std::time::Duration;

use serde::Serialize;

use crate::cli::{Cli, SessionCommand, SessionListArgs, SessionShowArgs, SessionStopArgs, SessionTailArgs};
use crate::context::BratContext;
use crate::error::BratError;
use crate::output::{output_success, print_human};

/// Session info for list/show output.
#[derive(Debug, Serialize)]
pub struct SessionInfo {
    /// Session ID.
    pub session_id: String,
    /// Associated task ID.
    pub task_id: String,
    /// Role executing the session.
    pub role: String,
    /// Session type (polecat/crew).
    pub session_type: String,
    /// Engine name.
    pub engine: String,
    /// Session state.
    pub state: String,
    /// Timestamp when session started (millis since epoch).
    pub started_ts: i64,
    /// Last heartbeat timestamp (millis since epoch).
    pub last_heartbeat_ts: Option<i64>,
    /// Heartbeat age in milliseconds (computed).
    pub heartbeat_age_ms: Option<i64>,
    /// Path to worktree.
    pub worktree: String,
    /// Process ID.
    pub pid: Option<u32>,
}

/// Output for session list command.
#[derive(Debug, Serialize)]
pub struct SessionListOutput {
    /// List of active sessions.
    pub sessions: Vec<SessionInfo>,
    /// Total count.
    pub total: usize,
}

/// Output for session show command.
#[derive(Debug, Serialize)]
pub struct SessionShowOutput {
    /// Session details.
    pub session: SessionInfo,
}

/// Output for session stop command.
#[derive(Debug, Serialize)]
pub struct SessionStopOutput {
    /// Session ID that was stopped.
    pub session_id: String,
    /// Reason for stopping.
    pub reason: String,
    /// Whether exit was posted to Gritee.
    pub exit_posted: bool,
}

/// Output for session tail command.
#[derive(Debug, Serialize)]
pub struct SessionTailOutput {
    /// Session ID.
    pub session_id: String,
    /// Number of lines returned.
    pub lines_count: usize,
    /// The log lines.
    pub lines: Vec<String>,
    /// Whether there were more lines available.
    pub truncated: bool,
}

/// Run the session command.
pub fn run(cli: &Cli, cmd: &SessionCommand) -> Result<(), BratError> {
    match cmd {
        SessionCommand::List(args) => run_list(cli, args),
        SessionCommand::Show(args) => run_show(cli, args),
        SessionCommand::Stop(args) => run_stop(cli, args),
        SessionCommand::Tail(args) => run_tail(cli, args),
    }
}

/// Run the session list command.
fn run_list(cli: &Cli, args: &SessionListArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;
    ctx.require_initialized()?;
    ctx.require_gritee_initialized()?;

    let client = ctx.gritee_client();
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    // Get sessions, optionally filtered by task
    let sessions = client.session_list(args.task.as_deref())?;

    let session_infos: Vec<SessionInfo> = sessions
        .into_iter()
        .map(|s| {
            let heartbeat_age_ms = s.last_heartbeat_ts.map(|ts| now_ms - ts);
            SessionInfo {
                session_id: s.session_id,
                task_id: s.task_id,
                role: s.role.as_str().to_string(),
                session_type: s.session_type.as_str().to_string(),
                engine: s.engine,
                state: format!("{:?}", s.status).to_lowercase(),
                started_ts: s.started_ts,
                last_heartbeat_ts: s.last_heartbeat_ts,
                heartbeat_age_ms,
                worktree: s.worktree,
                pid: s.pid,
            }
        })
        .collect();

    let total = session_infos.len();

    if !cli.json && !cli.quiet {
        if session_infos.is_empty() {
            print_human(cli, "No active sessions");
        } else {
            println!("Active Sessions ({}):", total);
            for s in &session_infos {
                let heartbeat_str = match s.heartbeat_age_ms {
                    Some(age_ms) if age_ms < 60_000 => format!("{}s ago", age_ms / 1000),
                    Some(age_ms) => format!("{}m ago", age_ms / 60_000),
                    None => "never".to_string(),
                };
                println!(
                    "  {}  {}  {}/{}  {}  heartbeat {}",
                    s.session_id, s.task_id, s.role, s.session_type, s.state, heartbeat_str
                );
            }
        }
    }

    let output = SessionListOutput {
        sessions: session_infos,
        total,
    };

    output_success(cli, output);
    Ok(())
}

/// Run the session show command.
fn run_show(cli: &Cli, args: &SessionShowArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;
    ctx.require_initialized()?;
    ctx.require_gritee_initialized()?;

    let client = ctx.gritee_client();
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    let session = client.session_get(&args.session_id)?;

    let heartbeat_age_ms = session.last_heartbeat_ts.map(|ts| now_ms - ts);
    let session_info = SessionInfo {
        session_id: session.session_id.clone(),
        task_id: session.task_id.clone(),
        role: session.role.as_str().to_string(),
        session_type: session.session_type.as_str().to_string(),
        engine: session.engine.clone(),
        state: format!("{:?}", session.status).to_lowercase(),
        started_ts: session.started_ts,
        last_heartbeat_ts: session.last_heartbeat_ts,
        heartbeat_age_ms,
        worktree: session.worktree.clone(),
        pid: session.pid,
    };

    if !cli.json && !cli.quiet {
        println!("Session: {}", session_info.session_id);
        println!("  Task:       {}", session_info.task_id);
        println!("  Role:       {}", session_info.role);
        println!("  Type:       {}", session_info.session_type);
        println!("  Engine:     {}", session_info.engine);
        println!("  State:      {}", session_info.state);
        println!("  Started:    {}", session_info.started_ts);
        if let Some(ts) = session_info.last_heartbeat_ts {
            let age_str = match session_info.heartbeat_age_ms {
                Some(age_ms) if age_ms < 60_000 => format!("{}s ago", age_ms / 1000),
                Some(age_ms) => format!("{}m ago", age_ms / 60_000),
                None => "unknown".to_string(),
            };
            println!("  Heartbeat:  {} ({})", ts, age_str);
        }
        if !session_info.worktree.is_empty() {
            println!("  Worktree:   {}", session_info.worktree);
        }
        if let Some(pid) = session_info.pid {
            println!("  PID:        {}", pid);
        }
    }

    let output = SessionShowOutput {
        session: session_info,
    };

    output_success(cli, output);
    Ok(())
}

/// Run the session stop command.
fn run_stop(cli: &Cli, args: &SessionStopArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;
    ctx.require_initialized()?;
    ctx.require_gritee_initialized()?;

    let client = ctx.gritee_client();

    // Record the session exit in Grite
    client.session_exit(&args.session_id, 0, &args.reason, None)?;

    if !cli.json && !cli.quiet {
        print_human(
            cli,
            &format!(
                "Stopped session {} (reason: {})",
                args.session_id, args.reason
            ),
        );
    }

    let output = SessionStopOutput {
        session_id: args.session_id.clone(),
        reason: args.reason.clone(),
        exit_posted: true,
    };

    output_success(cli, output);
    Ok(())
}

/// Run the session tail command.
fn run_tail(cli: &Cli, args: &SessionTailArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;
    ctx.require_initialized()?;
    ctx.require_gritee_initialized()?;

    let client = ctx.gritee_client();

    // Get session to find log blob ref
    let session = client.session_get(&args.session_id)?;

    // Check if session has logs available
    let last_output_ref = match &session.last_output_ref {
        Some(ref_str) => ref_str.clone(),
        None => {
            print_human(cli, "No logs available for this session");
            let output = SessionTailOutput {
                session_id: args.session_id.clone(),
                lines_count: 0,
                lines: Vec::new(),
                truncated: false,
            };
            output_success(cli, output);
            return Ok(());
        }
    };

    // Read the log content from git blob
    let log_content = read_blob(&ctx.repo_root, &last_output_ref)?;

    // Split into lines and get the last N
    let all_lines: Vec<&str> = log_content.lines().collect();
    let total_lines = all_lines.len();
    let truncated = total_lines > args.lines;
    let start = total_lines.saturating_sub(args.lines);
    let lines: Vec<String> = all_lines[start..].iter().map(|s| s.to_string()).collect();

    // Output lines
    if !cli.json && !cli.quiet {
        for line in &lines {
            println!("{}", line);
        }
    }

    // Follow mode
    if args.follow {
        run_tail_follow(cli, &ctx.repo_root, &args.session_id, &client)?;
    } else {
        let output = SessionTailOutput {
            session_id: args.session_id.clone(),
            lines_count: lines.len(),
            lines,
            truncated,
        };
        output_success(cli, output);
    }

    Ok(())
}

/// Follow mode for session tail - polls for new log content.
fn run_tail_follow(
    cli: &Cli,
    repo_root: &std::path::Path,
    session_id: &str,
    client: &libbrat_grite::GriteeClient,
) -> Result<(), BratError> {
    let poll_interval = Duration::from_secs(1);
    let mut last_ref: Option<String> = None;
    let mut last_line_count: usize = 0;

    loop {
        // Get current session state
        let session = match client.session_get(session_id) {
            Ok(s) => s,
            Err(_) => {
                // Session might be gone, stop following
                if !cli.json && !cli.quiet {
                    println!("\n[session exited]");
                }
                break;
            }
        };

        // Check for new logs
        if let Some(ref ref_str) = session.last_output_ref {
            // If ref changed or this is first check, read new content
            if last_ref.as_ref() != Some(ref_str) || last_ref.is_none() {
                if let Ok(log_content) = read_blob(repo_root, ref_str) {
                    let lines: Vec<&str> = log_content.lines().collect();

                    // Output only new lines
                    for line in lines.iter().skip(last_line_count) {
                        if !cli.json {
                            println!("{}", line);
                        }
                    }

                    last_line_count = lines.len();
                    last_ref = Some(ref_str.clone());
                }
            }
        }

        // Check if session has exited
        if session.status == libbrat_grite::SessionStatus::Exit {
            if !cli.json && !cli.quiet {
                println!("\n[session exited]");
            }
            break;
        }

        std::thread::sleep(poll_interval);
    }

    Ok(())
}

/// Read blob content from git.
fn read_blob(repo_root: &std::path::Path, blob_ref: &str) -> Result<String, BratError> {
    // The blob ref might be in format "sha256:xxxx" or just a git hash
    let hash = if let Some(stripped) = blob_ref.strip_prefix("sha256:") {
        stripped
    } else {
        blob_ref
    };

    let output = Command::new("git")
        .args(["cat-file", "blob", hash])
        .current_dir(repo_root)
        .output()
        .map_err(|e| BratError::GriteeCommandFailed(format!("failed to read blob: {}", e)))?;

    if !output.status.success() {
        return Err(BratError::GriteeCommandFailed(format!(
            "blob not found: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
