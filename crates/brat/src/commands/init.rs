use std::path::Path;
use std::process::Command;

use libbrat_config::BratConfig;
use serde::Serialize;

use crate::agents_md::BRAT_AGENTS_SECTION;
use crate::cli::{Cli, InitArgs};
use crate::context::BratContext;
use crate::error::BratError;
use crate::output::{output_success, print_human};

/// Action taken for AGENTS.md
#[derive(Clone, Copy)]
enum AgentsMdAction {
    Created,
    Updated,
    Skipped,
    Disabled,
}

impl AgentsMdAction {
    fn as_str(&self) -> &'static str {
        match self {
            AgentsMdAction::Created => "created",
            AgentsMdAction::Updated => "updated",
            AgentsMdAction::Skipped => "skipped",
            AgentsMdAction::Disabled => "disabled",
        }
    }
}

/// Output of the init command.
#[derive(Debug, Serialize)]
pub struct InitOutput {
    /// Path to the repository root.
    pub repo_root: String,

    /// Path to the .brat directory.
    pub brat_dir: String,

    /// Path to the config file (if created).
    pub config_path: Option<String>,

    /// Whether gritee was initialized.
    pub gritee_initialized: bool,

    /// Actor ID from gritee (if available).
    pub gritee_actor_id: Option<String>,

    /// Action taken for AGENTS.md (created, updated, skipped, disabled).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agents_md_action: Option<String>,

    /// Whether KB was initialized.
    pub kb_initialized: bool,

    /// Whether bootstrap ran (only for existing repos).
    pub bootstrap_ran: bool,

    /// Bootstrap consistency score (0-100) if bootstrap ran.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bootstrap_score: Option<u8>,

    /// Number of inconsistencies found during bootstrap.
    pub bootstrap_inconsistencies: usize,
}

/// Run the init command.
pub async fn run(cli: &Cli, args: &InitArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;

    // Check if already initialized
    if ctx.is_initialized() && !args.no_config {
        return Err(BratError::AlreadyInitialized);
    }

    // Initialize gritee if needed (pass --no-agents-md if set)
    let (gritee_initialized, gritee_actor_id) = init_gritee(&ctx, args.no_agents_md)?;

    // Handle AGENTS.md (add brat section)
    let agents_md_action = if args.no_agents_md {
        AgentsMdAction::Disabled
    } else {
        handle_agents_md(&ctx.repo_root)?
    };

    // Create .brat/config.toml unless --no-config
    let config = BratConfig::default();
    let config_path = if !args.no_config {
        config.save(&ctx.config_path)?;
        Some(ctx.config_path.display().to_string())
    } else {
        None
    };

    // Ensure .brat/.gitignore tracks notes but ignores runtime state
    ensure_brat_gitignore(&ctx.repo_root)?;

    // Initialize knowledge base
    let kb_initialized = init_kb(&ctx.repo_root)?;

    // Install git hooks unless --no-hooks
    if !args.no_hooks {
        install_git_hooks(&ctx.repo_root, config.kb.min_consistency_score)?;
    }

    // Check if repo has existing commits and run bootstrap
    let (bootstrap_ran, bootstrap_score, bootstrap_inconsistencies) =
        if kb_initialized && git_has_commits(&ctx.repo_root)? {
            println!("Detected existing repository. Starting auto-bootstrap...");
            match run_bootstrap(&ctx.repo_root, config.bootstrap.max_iterations).await {
                Ok(result) => {
                    let score = result.consistency_score;
                    let inconsistencies = result.inconsistencies.len();

                    if !result.consistent {
                        println!(
                            "Bootstrap complete with {} inconsistencies (score: {})",
                            inconsistencies, score
                        );
                        for inc in &result.inconsistencies {
                            println!("  - [{:?}] {}", inc.severity, inc.description);
                        }
                        println!("Review and edit notes via the UI or `brat kb edit`.");
                    } else {
                        println!("Bootstrap complete. KB is consistent (score: {}).", score);
                    }

                    (true, Some(score), inconsistencies)
                }
                Err(e) => {
                    eprintln!("Bootstrap failed: {}", e);
                    (true, None, 0)
                }
            }
        } else {
            (false, None, 0)
        };

    // TODO: Start bratd unless --no-daemon
    // TODO: Create tmux control room unless --no-tmux

    let output = InitOutput {
        repo_root: ctx.repo_root.display().to_string(),
        brat_dir: ctx.brat_dir.display().to_string(),
        config_path,
        gritee_initialized,
        gritee_actor_id,
        agents_md_action: Some(agents_md_action.as_str().to_string()),
        kb_initialized,
        bootstrap_ran,
        bootstrap_score,
        bootstrap_inconsistencies,
    };

    if !cli.json {
        print_human(cli, &format!("Initialized brat in {}", ctx.repo_root.display()));
        if gritee_initialized {
            if let Some(ref actor_id) = output.gritee_actor_id {
                print_human(cli, &format!("Grite actor: {}", actor_id));
            }
        }
        // Print AGENTS.md status
        match agents_md_action {
            AgentsMdAction::Created => {
                print_human(cli, "Created AGENTS.md with brat instructions");
            }
            AgentsMdAction::Updated => {
                print_human(cli, "Updated AGENTS.md with brat section");
            }
            AgentsMdAction::Skipped => {
                print_human(cli, "AGENTS.md already contains brat section");
            }
            AgentsMdAction::Disabled => {}
        }
        if kb_initialized {
            print_human(cli, "KB notes are in .brat/notes/ and tracked by git.");
            print_human(cli, "Run `brat kb check` after editing notes to sync and verify consistency.");
        }
        if !args.no_hooks {
            print_human(cli, "Installed git pre-commit hook for KB consistency.");
        }
    }

    output_success(cli, output);
    Ok(())
}

/// Initialize gritee in the repository.
///
/// Calls `gritee init` as a subprocess and parses the output.
fn init_gritee(ctx: &BratContext, no_agents_md: bool) -> Result<(bool, Option<String>), BratError> {
    // Check if grite is already initialized by looking for .git/gritee/
    let gritee_dir = ctx.git_dir.join("gritee");
    if gritee_dir.exists() {
        // Already initialized, try to get the actor ID
        let actor_id = get_gritee_actor_id(ctx)?;
        return Ok((false, actor_id));
    }

    // Run gritee init
    let mut cmd = Command::new("gritee");
    cmd.arg("init").arg("--json");
    if no_agents_md {
        cmd.arg("--no-agents-md");
    }
    cmd.current_dir(&ctx.repo_root);

    let output = cmd
        .output()
        .map_err(|e| BratError::GriteeInitFailed(format!("failed to run gritee: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(BratError::GriteeInitFailed(stderr.to_string()));
    }

    // Parse the JSON output to get the actor ID
    let stdout = String::from_utf8_lossy(&output.stdout);
    let actor_id = parse_gritee_init_output(&stdout);

    Ok((true, actor_id))
}

/// Get the current gritee actor ID.
fn get_gritee_actor_id(ctx: &BratContext) -> Result<Option<String>, BratError> {
    let output = Command::new("gritee")
        .args(["actor", "current", "--json"])
        .current_dir(&ctx.repo_root)
        .output()
        .map_err(|e| BratError::GriteeCommandFailed(format!("failed to run gritee: {}", e)))?;

    if !output.status.success() {
        return Ok(None);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_gritee_actor_output(&stdout))
}

/// Parse gritee init JSON output to extract actor_id.
fn parse_gritee_init_output(output: &str) -> Option<String> {
    // Try to parse as JSON and extract actor_id
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(output) {
        if let Some(data) = json.get("data") {
            if let Some(actor_id) = data.get("actor_id") {
                return actor_id.as_str().map(|s| s.to_string());
            }
        }
    }
    None
}

/// Parse gritee actor current JSON output to extract actor_id.
fn parse_gritee_actor_output(output: &str) -> Option<String> {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(output) {
        if let Some(data) = json.get("data") {
            if let Some(actor_id) = data.get("actor_id") {
                return actor_id.as_str().map(|s| s.to_string());
            }
        }
    }
    None
}

/// Initialize the knowledge base via zkb-lib.
fn init_kb(repo_root: &Path) -> Result<bool, BratError> {
    use libbrat_kb::KbService;

    match KbService::open(repo_root) {
        Ok(kb) => {
            // Create structure notes for product and architecture
            let _ = kb.upsert_product_notes(&[libbrat_kb::ProductNoteInput {
                title: "Product Overview".to_string(),
                body: "Index of all product requirements and features.".to_string(),
                tags: vec!["structure".to_string()],
                acceptance_criteria: vec![],
                priority: Some("P0".to_string()),
            }]);
            let _ = kb.upsert_architecture_notes(&[libbrat_kb::ArchitectureNoteInput {
                title: "Architecture Overview".to_string(),
                body: "Index of all architecture components and design decisions.".to_string(),
                tags: vec!["structure".to_string()],
                components: vec![],
                interfaces: vec![],
                file_paths: vec![],
            }]);
            Ok(true)
        }
        Err(e) => {
            eprintln!("Warning: failed to initialize KB: {}", e);
            Ok(false)
        }
    }
}

/// Check if a git repository has existing commits.
fn git_has_commits(repo_root: &Path) -> Result<bool, BratError> {
    let output = Command::new("git")
        .args(["rev-list", "--count", "HEAD"])
        .current_dir(repo_root)
        .output()
        .map_err(|e| BratError::Other(format!("failed to run git: {}", e)))?;

    if !output.status.success() {
        return Ok(false);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    match stdout.trim().parse::<u32>() {
        Ok(count) => Ok(count > 0),
        Err(_) => Ok(false),
    }
}

/// Result of the bootstrap process.
#[derive(Debug)]
struct BootstrapResult {
    consistent: bool,
    consistency_score: u8,
    inconsistencies: Vec<libbrat_kb::Inconsistency>,
}

/// Run the auto-bootstrap process.
async fn run_bootstrap(
    repo_root: &Path,
    _max_iterations: u32,
) -> Result<BootstrapResult, BratError> {
    use libbrat_engine::{scan_codebase, infer_product_notes, infer_architecture_notes};
    use libbrat_kb::KbService;

    let scan = scan_codebase(repo_root)
        .map_err(|e| BratError::Other(format!("scan failed: {}", e)))?;

    let product_notes = infer_product_notes(&scan);
    let arch_notes = infer_architecture_notes(&scan, &product_notes);

    let kb = KbService::open(repo_root)
        .map_err(|e| BratError::Other(format!("KB open failed: {}", e)))?;

    // Upsert inferred notes
    let product_inputs: Vec<_> = product_notes
        .into_iter()
        .map(|n| libbrat_kb::ProductNoteInput {
            title: n.title,
            body: n.body,
            tags: n.tags,
            acceptance_criteria: n.acceptance_criteria,
            priority: Some(n.priority),
        })
        .collect();

    let arch_inputs: Vec<_> = arch_notes
        .into_iter()
        .map(|n| libbrat_kb::ArchitectureNoteInput {
            title: n.title,
            body: n.body,
            tags: n.tags,
            components: n.components,
            interfaces: n.interfaces,
            file_paths: n.file_paths,
        })
        .collect();

    kb.upsert_product_notes(&product_inputs)
        .await
        .map_err(|e| BratError::Other(format!("product upsert failed: {}", e)))?;
    kb.upsert_architecture_notes(&arch_inputs)
        .await
        .map_err(|e| BratError::Other(format!("arch upsert failed: {}", e)))?;

    // Run consistency check via KB service
    let check_result = kb.run_consistency_check()
        .await
        .map_err(|e| BratError::Other(format!("consistency check failed: {}", e)))?;

    Ok(BootstrapResult {
        consistent: check_result.inconsistencies.is_empty(),
        consistency_score: check_result.score(),
        inconsistencies: check_result.inconsistencies,
    })
}

/// Ensure .brat/.gitignore exists and root .gitignore does not blanket-ignore .brat.
fn ensure_brat_gitignore(repo_root: &Path) -> Result<(), BratError> {
    let brat_dir = repo_root.join(".brat");
    let brat_gitignore = brat_dir.join(".gitignore");

    if !brat_gitignore.exists() {
        std::fs::write(
            &brat_gitignore,
            "# zkb runtime state — rebuildable from notes/*.md\n.zkb/\n",
        )
        .map_err(|e| BratError::Other(format!("failed to write .brat/.gitignore: {}", e)))?;
    }

    // Update root .gitignore to remove blanket `.brat` ignore so .brat/.gitignore is read
    let root_gitignore = repo_root.join(".gitignore");
    if root_gitignore.exists() {
        let content = std::fs::read_to_string(&root_gitignore)
            .map_err(|e| BratError::Other(format!("failed to read .gitignore: {}", e)))?;
        let lines: Vec<&str> = content.lines().collect();
        let has_blanket_brat = lines.iter().any(|l| l.trim() == ".brat");
        if has_blanket_brat {
            let updated: Vec<String> = lines
                .into_iter()
                .map(|l| {
                    if l.trim() == ".brat" {
                        "# .brat".to_string()
                    } else {
                        l.to_string()
                    }
                })
                .collect();
            std::fs::write(
                &root_gitignore,
                updated.join("\n") + "\n",
            )
            .map_err(|e| BratError::Other(format!("failed to update .gitignore: {}", e)))?;
        }
    }

    Ok(())
}

/// Install git pre-commit hook that runs `brat kb check`.
fn install_git_hooks(repo_root: &Path, min_score: u8) -> Result<(), BratError> {
    let hooks_dir = repo_root.join(".git").join("hooks");
    if !hooks_dir.exists() {
        return Ok(());
    }

    let pre_commit_path = hooks_dir.join("pre-commit");
    let guard_start = "# >>> brat kb pre-commit guard";
    let guard_end = "# <<< brat kb pre-commit guard";

    let hook_block = format!(
        r#"{guard_start}
# Run brat kb check and block commits below consistency threshold
brat kb check --min-score {min_score} || {{
    echo ""
    echo "Commit blocked: KB consistency score below {min_score}."
    echo "Run 'brat kb edit <slug>' to fix inconsistencies, or 'brat kb check' for details."
    exit 1
}}
{guard_end}
"#,
        guard_start = guard_start,
        guard_end = guard_end,
        min_score = min_score
    );

    let existing = if pre_commit_path.exists() {
        std::fs::read_to_string(&pre_commit_path)
            .map_err(|e| BratError::Other(format!("failed to read pre-commit hook: {}", e)))?
    } else {
        String::new()
    };

    // If guard already exists, replace it; otherwise append
    let new_content = if existing.contains(guard_start) {
        let mut result = String::new();
        let mut in_guard = false;
        for line in existing.lines() {
            if line.trim() == guard_start {
                in_guard = true;
                continue;
            }
            if line.trim() == guard_end {
                in_guard = false;
                continue;
            }
            if !in_guard {
                result.push_str(line);
                result.push('\n');
            }
        }
        result + &hook_block
    } else {
        let prefix = if existing.trim().is_empty() {
            "#!/bin/sh\n".to_string()
        } else {
            existing.trim_end().to_string() + "\n"
        };
        prefix + "\n" + &hook_block
    };

    std::fs::write(&pre_commit_path, new_content)
        .map_err(|e| BratError::Other(format!("failed to write pre-commit hook: {}", e)))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&pre_commit_path)
            .map_err(|e| BratError::Other(format!("failed to get hook metadata: {}", e)))?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&pre_commit_path, perms)
            .map_err(|e| BratError::Other(format!("failed to chmod pre-commit hook: {}", e)))?;
    }

    Ok(())
}

/// Handle AGENTS.md - add brat section if not already present.
fn handle_agents_md(repo_root: &Path) -> Result<AgentsMdAction, BratError> {
    let agents_md_path = repo_root.join("AGENTS.md");

    if agents_md_path.exists() {
        let content = std::fs::read_to_string(&agents_md_path)
            .map_err(|e| BratError::Other(format!("failed to read AGENTS.md: {}", e)))?;

        if content.contains("## Brat") {
            return Ok(AgentsMdAction::Skipped);
        }

        // Append Brat section
        let updated = format!("{}\n\n{}", content, BRAT_AGENTS_SECTION);
        std::fs::write(&agents_md_path, updated)
            .map_err(|e| BratError::Other(format!("failed to update AGENTS.md: {}", e)))?;
        Ok(AgentsMdAction::Updated)
    } else {
        // Create new with Brat section
        std::fs::write(&agents_md_path, BRAT_AGENTS_SECTION)
            .map_err(|e| BratError::Other(format!("failed to create AGENTS.md: {}", e)))?;
        Ok(AgentsMdAction::Created)
    }
}
