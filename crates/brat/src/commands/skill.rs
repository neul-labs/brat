//! Skill command handlers.

use crate::cli::{Cli, SkillCommand, SkillInstallArgs, SkillListArgs};
use crate::context::BratContext;
use crate::error::BratError;
use crate::output::{output_success, print_human};

/// Run a skill subcommand.
pub fn run(cli: &Cli, cmd: &SkillCommand) -> Result<(), BratError> {
    match cmd {
        SkillCommand::Install(args) => run_install(cli, args),
        SkillCommand::List(args) => run_list(cli, args),
    }
}

fn run_install(cli: &Cli, args: &SkillInstallArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;
    ctx.require_initialized()?;

    print_human(cli, "Installing brat skills...");

    if args.force {
        print_human(cli, "Force reinstall enabled.");
    }

    // TODO: Integrate with brat_skills
    if cli.json {
        #[derive(serde::Serialize)]
        struct Output {
            installed: Vec<String>,
        }
        output_success(cli, Output { installed: vec![] });
    } else {
        print_human(cli, "Skill installation integration pending.");
    }

    Ok(())
}

fn run_list(cli: &Cli, _args: &SkillListArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;
    ctx.require_initialized()?;

    print_human(cli, "Listing embedded skills...");

    // TODO: Integrate with brat_skills
    if cli.json {
        #[derive(serde::Serialize)]
        struct Output {
            skills: Vec<String>,
        }
        output_success(cli, Output { skills: vec![] });
    } else {
        print_human(cli, "No skills (skill integration pending).");
    }

    Ok(())
}
