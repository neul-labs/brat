use libbrat_grite::{DependencyType, TaskStatus};
use serde::Serialize;

use crate::cli::{
    Cli, TaskCommand, TaskCreateArgs, TaskDepAddArgs, TaskDepCommand, TaskDepListArgs,
    TaskDepRemoveArgs, TaskDepTopoArgs, TaskUpdateArgs,
};
use crate::context::BratContext;
use crate::error::BratError;
use crate::output::{output_success, print_human};

/// Output of the task create command.
#[derive(Debug, Serialize)]
pub struct TaskCreateOutput {
    /// Brat task ID.
    pub task_id: String,

    /// Grite's internal issue ID.
    pub gritee_issue_id: String,

    /// Parent convoy ID.
    pub convoy_id: String,

    /// Task title.
    pub title: String,

    /// Task status.
    pub status: String,
}

/// Output of the task update command.
#[derive(Debug, Serialize)]
pub struct TaskUpdateOutput {
    /// Task ID that was updated.
    pub task_id: String,

    /// New status.
    pub status: String,
}

/// Output of task dep add/remove commands.
#[derive(Debug, Serialize)]
pub struct TaskDepModifyOutput {
    /// Task ID.
    pub task_id: String,

    /// Target task ID.
    pub target: String,

    /// Dependency type.
    pub dep_type: String,

    /// Action taken.
    pub action: String,
}

/// Output of task dep list command.
#[derive(Debug, Serialize)]
pub struct TaskDepListOutput {
    /// Task ID.
    pub task_id: String,

    /// Direction of dependencies.
    pub direction: String,

    /// List of dependencies.
    pub deps: Vec<TaskDepEntry>,
}

/// A dependency entry.
#[derive(Debug, Serialize)]
pub struct TaskDepEntry {
    /// Target issue ID.
    pub issue_id: String,

    /// Dependency type.
    pub dep_type: String,

    /// Target title.
    pub title: String,
}

/// Output of task dep topo command.
#[derive(Debug, Serialize)]
pub struct TaskDepTopoOutput {
    /// Tasks in topological order.
    pub tasks: Vec<TaskTopoEntry>,
}

/// A task entry in topological order.
#[derive(Debug, Serialize)]
pub struct TaskTopoEntry {
    /// Issue ID.
    pub issue_id: String,

    /// Task title.
    pub title: String,

    /// Task state.
    pub state: String,

    /// Labels.
    pub labels: Vec<String>,
}

/// Run the task command.
pub fn run(cli: &Cli, cmd: &TaskCommand) -> Result<(), BratError> {
    match cmd {
        TaskCommand::Create(args) => run_create(cli, args),
        TaskCommand::Update(args) => run_update(cli, args),
        TaskCommand::Dep(dep_cmd) => run_dep(cli, dep_cmd),
    }
}

/// Run task dep subcommands.
fn run_dep(cli: &Cli, cmd: &TaskDepCommand) -> Result<(), BratError> {
    match cmd {
        TaskDepCommand::Add(args) => run_dep_add(cli, args),
        TaskDepCommand::Remove(args) => run_dep_remove(cli, args),
        TaskDepCommand::List(args) => run_dep_list(cli, args),
        TaskDepCommand::Topo(args) => run_dep_topo(cli, args),
    }
}

/// Run the task create command.
fn run_create(cli: &Cli, args: &TaskCreateArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;

    // Require both brat and gritee to be initialized
    ctx.require_initialized()?;
    ctx.require_gritee_initialized()?;

    let client = ctx.gritee_client();
    let task = client.task_create(&args.convoy, &args.title, args.body.as_deref())?;

    let output = TaskCreateOutput {
        task_id: task.task_id.clone(),
        gritee_issue_id: task.gritee_issue_id,
        convoy_id: task.convoy_id,
        title: task.title,
        status: format!("{:?}", task.status).to_lowercase(),
    };

    if !cli.json {
        print_human(cli, &format!("Created task {}", task.task_id));
    }

    output_success(cli, output);
    Ok(())
}

/// Run the task update command.
fn run_update(cli: &Cli, args: &TaskUpdateArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;

    // Require both brat and gritee to be initialized
    ctx.require_initialized()?;
    ctx.require_gritee_initialized()?;

    // Parse the status argument
    let new_status = parse_task_status(&args.status)?;

    let client = ctx.gritee_client();
    client.task_update_status_with_options(&args.task_id, new_status, args.force)?;

    let output = TaskUpdateOutput {
        task_id: args.task_id.clone(),
        status: format!("{:?}", new_status).to_lowercase(),
    };

    if !cli.json {
        let msg = if args.force {
            format!("Force-updated task {} to {}", args.task_id, args.status)
        } else {
            format!("Updated task {} to {}", args.task_id, args.status)
        };
        print_human(cli, &msg);
    }

    output_success(cli, output);
    Ok(())
}

/// Parse a status string into a TaskStatus.
fn parse_task_status(s: &str) -> Result<TaskStatus, BratError> {
    match s.to_lowercase().as_str() {
        "queued" => Ok(TaskStatus::Queued),
        "running" => Ok(TaskStatus::Running),
        "blocked" => Ok(TaskStatus::Blocked),
        "needs-review" | "needs_review" | "needsreview" => Ok(TaskStatus::NeedsReview),
        "merged" => Ok(TaskStatus::Merged),
        "dropped" => Ok(TaskStatus::Dropped),
        _ => Err(BratError::GriteeCommandFailed(format!(
            "invalid status '{}': expected one of queued, running, blocked, needs-review, merged, dropped",
            s
        ))),
    }
}

/// Parse a dependency type string.
fn parse_dep_type(s: &str) -> Result<DependencyType, BratError> {
    DependencyType::from_str(s).ok_or_else(|| {
        BratError::GriteeCommandFailed(format!(
            "invalid dependency type '{}': expected one of blocks, depends_on, related_to",
            s
        ))
    })
}

/// Run the task dep add command.
fn run_dep_add(cli: &Cli, args: &TaskDepAddArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;
    ctx.require_initialized()?;
    ctx.require_gritee_initialized()?;

    let dep_type = parse_dep_type(&args.dep_type)?;

    // Get the gritee issue IDs for the task IDs
    let client = ctx.gritee_client();
    let task = client.task_get(&args.task_id)?;
    let target_task = client.task_get(&args.target)?;

    client.task_dep_add(&task.gritee_issue_id, &target_task.gritee_issue_id, dep_type)?;

    let output = TaskDepModifyOutput {
        task_id: args.task_id.clone(),
        target: args.target.clone(),
        dep_type: args.dep_type.clone(),
        action: "added".to_string(),
    };

    if !cli.json {
        print_human(
            cli,
            &format!(
                "Added {} dependency: {} -> {}",
                args.dep_type, args.task_id, args.target
            ),
        );
    }

    output_success(cli, output);
    Ok(())
}

/// Run the task dep remove command.
fn run_dep_remove(cli: &Cli, args: &TaskDepRemoveArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;
    ctx.require_initialized()?;
    ctx.require_gritee_initialized()?;

    let dep_type = parse_dep_type(&args.dep_type)?;

    // Get the gritee issue IDs for the task IDs
    let client = ctx.gritee_client();
    let task = client.task_get(&args.task_id)?;
    let target_task = client.task_get(&args.target)?;

    client.task_dep_remove(&task.gritee_issue_id, &target_task.gritee_issue_id, dep_type)?;

    let output = TaskDepModifyOutput {
        task_id: args.task_id.clone(),
        target: args.target.clone(),
        dep_type: args.dep_type.clone(),
        action: "removed".to_string(),
    };

    if !cli.json {
        print_human(
            cli,
            &format!(
                "Removed {} dependency: {} -> {}",
                args.dep_type, args.task_id, args.target
            ),
        );
    }

    output_success(cli, output);
    Ok(())
}

/// Run the task dep list command.
fn run_dep_list(cli: &Cli, args: &TaskDepListArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;
    ctx.require_initialized()?;
    ctx.require_gritee_initialized()?;

    let client = ctx.gritee_client();
    let task = client.task_get(&args.task_id)?;

    let deps = client.task_dep_list(&task.gritee_issue_id, args.reverse)?;

    let direction = if args.reverse {
        "dependents"
    } else {
        "dependencies"
    };

    let output = TaskDepListOutput {
        task_id: args.task_id.clone(),
        direction: direction.to_string(),
        deps: deps
            .iter()
            .map(|d| TaskDepEntry {
                issue_id: d.issue_id.clone(),
                dep_type: d.dep_type.to_string(),
                title: d.title.clone(),
            })
            .collect(),
    };

    if !cli.json {
        if deps.is_empty() {
            print_human(cli, &format!("No {} for task {}", direction, args.task_id));
        } else {
            print_human(
                cli,
                &format!("{} for task {}:", direction.to_uppercase(), args.task_id),
            );
            for dep in &deps {
                print_human(cli, &format!("  {} [{}]: {}", dep.issue_id, dep.dep_type, dep.title));
            }
        }
    }

    output_success(cli, output);
    Ok(())
}

/// Run the task dep topo command.
fn run_dep_topo(cli: &Cli, args: &TaskDepTopoArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;
    ctx.require_initialized()?;
    ctx.require_gritee_initialized()?;

    let client = ctx.gritee_client();

    // Build label filter for convoy if specified
    let label = args.convoy.as_ref().map(|c| format!("convoy:{}", c));
    let issues = client.task_topo_order(label.as_deref())?;

    // Filter to only task issues
    let tasks: Vec<_> = issues
        .iter()
        .filter(|i| i.labels.iter().any(|l| l == "type:task"))
        .collect();

    let output = TaskDepTopoOutput {
        tasks: tasks
            .iter()
            .map(|t| TaskTopoEntry {
                issue_id: t.issue_id.clone(),
                title: t.title.clone(),
                state: t.state.clone(),
                labels: t.labels.clone(),
            })
            .collect(),
    };

    if !cli.json {
        if tasks.is_empty() {
            print_human(cli, "No tasks found");
        } else {
            print_human(cli, "Tasks in topological order (ready-to-run first):");
            for (i, task) in tasks.iter().enumerate() {
                // Extract task ID from labels
                let task_id = task
                    .labels
                    .iter()
                    .find_map(|l| l.strip_prefix("task:"))
                    .unwrap_or(&task.issue_id);
                print_human(cli, &format!("  {}. {} - {}", i + 1, task_id, task.title));
            }
        }
    }

    output_success(cli, output);
    Ok(())
}
