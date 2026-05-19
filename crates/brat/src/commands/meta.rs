//! Meta command handlers.

use libbrat_engine::{Engine, MetaEngine, SpawnSpec};

use crate::cli::{
    Cli, MetaAskArgs, MetaCommand, MetaStartArgs, MetaStatusArgs, MetaStopArgs, MetaTailArgs,
};
use crate::context::BratContext;
use crate::error::BratError;
use crate::output::{output_error_json, output_success, print_human};

/// Run a mayor subcommand.
pub async fn run(cli: &Cli, cmd: &MetaCommand) -> Result<(), BratError> {
    match cmd {
        MetaCommand::Start(args) => run_start(cli, args).await,
        MetaCommand::Ask(args) => run_ask(cli, args).await,
        MetaCommand::Status(args) => run_status(cli, args).await,
        MetaCommand::Tail(args) => run_tail(cli, args).await,
        MetaCommand::Stop(args) => run_stop(cli, args).await,
    }
}

/// Start the mayor.
async fn run_start(cli: &Cli, args: &MetaStartArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;
    ctx.require_initialized()?;

    let engine = MetaEngine::new(ctx.repo_root.clone());

    // Check if already active
    if engine.is_active() {
        if cli.json {
            output_error_json(cli, "mayor_already_running", "Meta is already running");
        } else {
            print_human(cli, "Meta is already running. Use 'brat mayor stop' to stop it first.");
        }
        return Err(BratError::Other("mayor already running".to_string()));
    }

    // Build spawn spec
    let initial_message = args.message.clone().unwrap_or_default();
    let spec = SpawnSpec::new(&initial_message).working_dir(&ctx.repo_root);

    // Spawn the mayor (this makes the initial Claude call)
    print_human(cli, "Starting Meta (this may take a moment)...");

    let result = engine.spawn(spec).await.map_err(|e| {
        BratError::Other(format!("failed to start mayor: {}", e))
    })?;

    if cli.json {
        #[derive(serde::Serialize)]
        struct Output {
            session_id: String,
            status: String,
        }
        output_success(
            cli,
            Output {
                session_id: result.session_id,
                status: "started".to_string(),
            },
        );
    } else {
        print_human(cli, &format!("Meta started successfully!"));
        print_human(cli, &format!("Session ID: {}", result.session_id));
        print_human(cli, "");
        print_human(cli, "Use 'brat mayor ask <message>' to send instructions.");
        print_human(cli, "Use 'brat mayor tail' to view output.");
        print_human(cli, "Use 'brat mayor stop' when done.");
    }

    Ok(())
}

/// Send a message to the mayor.
async fn run_ask(cli: &Cli, args: &MetaAskArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;
    let engine = MetaEngine::new(ctx.repo_root.clone());

    if !engine.is_active() {
        if cli.json {
            output_error_json(cli, "mayor_not_running", "Meta is not running");
        } else {
            print_human(cli, "Meta is not running. Use 'brat mayor start' first.");
        }
        return Err(BratError::Other("mayor not running".to_string()));
    }

    print_human(cli, "Sending message to Meta...");

    // Send the message
    let response = engine.ask(&args.message).map_err(|e| {
        BratError::Other(format!("failed to send message: {}", e))
    })?;

    if cli.json {
        #[derive(serde::Serialize)]
        struct Output {
            message_sent: String,
            response_lines: Vec<String>,
        }
        output_success(
            cli,
            Output {
                message_sent: args.message.clone(),
                response_lines: response,
            },
        );
    } else {
        print_human(cli, "");
        print_human(cli, "Response:");
        print_human(cli, "─".repeat(40).as_str());
        for line in &response {
            print_human(cli, line);
        }
    }

    Ok(())
}

/// Check mayor status.
async fn run_status(cli: &Cli, _args: &MetaStatusArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;
    let engine = MetaEngine::new(ctx.repo_root.clone());

    let is_active = engine.is_active();
    let session_id = engine.current_session_id();

    if cli.json {
        #[derive(serde::Serialize)]
        struct Output {
            active: bool,
            session_id: Option<String>,
        }
        output_success(
            cli,
            Output {
                active: is_active,
                session_id,
            },
        );
    } else if is_active {
        print_human(cli, "Meta is running.");
        if let Some(id) = session_id {
            print_human(cli, &format!("Session ID: {}", id));
        }
    } else {
        print_human(cli, "Meta is not running.");
    }

    Ok(())
}

/// View mayor output.
async fn run_tail(cli: &Cli, args: &MetaTailArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;
    let engine = MetaEngine::new(ctx.repo_root.clone());

    if !engine.is_active() {
        if cli.json {
            output_error_json(cli, "mayor_not_running", "Meta is not running");
        } else {
            print_human(cli, "Meta is not running.");
        }
        return Err(BratError::Other("mayor not running".to_string()));
    }

    let lines = engine.tail(args.lines).map_err(|e| {
        BratError::Other(format!("failed to get output: {}", e))
    })?;

    if cli.json {
        #[derive(serde::Serialize)]
        struct Output {
            lines: Vec<String>,
            count: usize,
        }
        output_success(
            cli,
            Output {
                count: lines.len(),
                lines,
            },
        );
    } else {
        if lines.is_empty() {
            print_human(cli, "(no output yet)");
        } else {
            for line in &lines {
                print_human(cli, line);
            }
        }
    }

    Ok(())
}

/// Stop the mayor.
async fn run_stop(cli: &Cli, _args: &MetaStopArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;
    let engine = MetaEngine::new(ctx.repo_root.clone());

    if !engine.is_active() {
        if cli.json {
            output_error_json(cli, "mayor_not_running", "Meta is not running");
        } else {
            print_human(cli, "Meta is not running.");
        }
        return Ok(());
    }

    let session_id = engine.current_session_id().unwrap_or_default();

    engine.stop_session().map_err(|e| {
        BratError::Other(format!("failed to stop mayor: {}", e))
    })?;

    if cli.json {
        #[derive(serde::Serialize)]
        struct Output {
            status: String,
            session_id: String,
        }
        output_success(
            cli,
            Output {
                status: "stopped".to_string(),
                session_id,
            },
        );
    } else {
        print_human(cli, "Meta stopped.");
    }

    Ok(())
}
