//! Knowledge base command handlers.

use crate::cli::{Cli, KbCommand, KbSearchArgs, KbProductArgs, KbArchitectureArgs, KbScoreArgs, KbInconsistenciesArgs, KbCheckArgs, KbEditArgs};
use crate::context::BratContext;
use crate::error::BratError;
use crate::output::{output_success, print_human};

/// Run a kb subcommand.
pub fn run(cli: &Cli, cmd: &KbCommand) -> Result<(), BratError> {
    match cmd {
        KbCommand::Search(args) => run_search(cli, args),
        KbCommand::Product(args) => run_product(cli, args),
        KbCommand::Architecture(args) => run_architecture(cli, args),
        KbCommand::Score(args) => run_score(cli, args),
        KbCommand::Inconsistencies(args) => run_inconsistencies(cli, args),
        KbCommand::Check(args) => run_check(cli, args),
        KbCommand::Edit(args) => run_edit(cli, args),
    }
}

/// Run zkb operations on a fresh OS thread with its own runtime,
/// avoiding `!Sync` issues on the tokio async worker thread.
fn run_kb_thread<F, R>(f: F) -> Result<R, BratError>
where
    F: FnOnce() -> Result<R, BratError> + Send + 'static,
    R: Send + 'static,
{
    std::thread::spawn(f)
        .join()
        .map_err(|e| BratError::Other(format!("KB thread panicked: {:?}", e)))?
}

fn run_search(cli: &Cli, args: &KbSearchArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;
    ctx.require_initialized()?;
    let query = args.query.clone();
    let repo_root = ctx.repo_root.clone();

    let results = run_kb_thread(move || {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| BratError::Other(format!("Runtime creation failed: {}", e)))?;
        rt.block_on(async {
            let kb = libbrat_kb::KbService::open(&repo_root)
                .map_err(|e| BratError::Other(format!("KB open failed: {}", e)))?;
            kb.search(&query).await
                .map_err(|e| BratError::Other(format!("Search failed: {}", e)))
        })
    })?;

    if cli.json {
        #[derive(serde::Serialize)]
        struct Output {
            query: String,
            results: Vec<libbrat_kb::SearchResult>,
        }
        output_success(cli, Output {
            query: args.query.clone(),
            results,
        });
    } else {
        print_human(cli, &format!("Found {} results for '{}'", results.len(), args.query));
        for r in results {
            print_human(cli, &format!("  [{}] {} (score: {:.2})", r.note_type, r.title, r.score));
        }
    }

    Ok(())
}

fn run_product(cli: &Cli, _args: &KbProductArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;
    ctx.require_initialized()?;
    let repo_root = ctx.repo_root.clone();

    let notes = run_kb_thread(move || {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| BratError::Other(format!("Runtime creation failed: {}", e)))?;
        rt.block_on(async {
            let kb = libbrat_kb::KbService::open(&repo_root)
                .map_err(|e| BratError::Other(format!("KB open failed: {}", e)))?;
            kb.list_product_notes().await
                .map_err(|e| BratError::Other(format!("List product notes failed: {}", e)))
        })
    })?;

    if cli.json {
        #[derive(serde::Serialize)]
        struct Output {
            notes: Vec<libbrat_kb::ProductNoteSummary>,
        }
        output_success(cli, Output { notes });
    } else {
        print_human(cli, &format!("Found {} product notes", notes.len()));
        for n in notes {
            print_human(cli, &format!("  [{}] {}", n.slug, n.title));
        }
    }

    Ok(())
}

fn run_architecture(cli: &Cli, _args: &KbArchitectureArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;
    ctx.require_initialized()?;
    let repo_root = ctx.repo_root.clone();

    let notes = run_kb_thread(move || {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| BratError::Other(format!("Runtime creation failed: {}", e)))?;
        rt.block_on(async {
            let kb = libbrat_kb::KbService::open(&repo_root)
                .map_err(|e| BratError::Other(format!("KB open failed: {}", e)))?;
            kb.list_architecture_notes().await
                .map_err(|e| BratError::Other(format!("List architecture notes failed: {}", e)))
        })
    })?;

    if cli.json {
        #[derive(serde::Serialize)]
        struct Output {
            notes: Vec<libbrat_kb::ArchitectureNoteSummary>,
        }
        output_success(cli, Output { notes });
    } else {
        print_human(cli, &format!("Found {} architecture notes", notes.len()));
        for n in notes {
            print_human(cli, &format!("  [{}] {}", n.slug, n.title));
        }
    }

    Ok(())
}

fn run_score(cli: &Cli, _args: &KbScoreArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;
    ctx.require_initialized()?;
    let repo_root = ctx.repo_root.clone();

    let check = run_kb_thread(move || {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| BratError::Other(format!("Runtime creation failed: {}", e)))?;
        rt.block_on(async {
            let kb = libbrat_kb::KbService::open(&repo_root)
                .map_err(|e| BratError::Other(format!("KB open failed: {}", e)))?;
            kb.run_consistency_check().await
                .map_err(|e| BratError::Other(format!("Consistency check failed: {}", e)))
        })
    })?;

    if cli.json {
        #[derive(serde::Serialize)]
        struct Output {
            score: u8,
            product_arch_coverage: f64,
            arch_product_traceability: f64,
            file_component_mapping: f64,
            test_feature_coverage: f64,
            doc_component_parity: f64,
        }
        output_success(cli, Output {
            score: check.score(),
            product_arch_coverage: check.product_arch_coverage,
            arch_product_traceability: check.arch_product_traceability,
            file_component_mapping: check.file_component_mapping,
            test_feature_coverage: check.test_feature_coverage,
            doc_component_parity: check.doc_component_parity,
        });
    } else {
        print_human(cli, &format!("Consistency score: {}", check.score()));
        print_human(cli, &format!("  Product→Arch coverage: {:.0}%", check.product_arch_coverage * 100.0));
        print_human(cli, &format!("  Arch→Product traceability: {:.0}%", check.arch_product_traceability * 100.0));
    }

    Ok(())
}

fn run_inconsistencies(cli: &Cli, _args: &KbInconsistenciesArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;
    ctx.require_initialized()?;
    let repo_root = ctx.repo_root.clone();

    let inconsistencies = run_kb_thread(move || {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| BratError::Other(format!("Runtime creation failed: {}", e)))?;
        rt.block_on(async {
            let kb = libbrat_kb::KbService::open(&repo_root)
                .map_err(|e| BratError::Other(format!("KB open failed: {}", e)))?;
            kb.list_inconsistencies().await
                .map_err(|e| BratError::Other(format!("List inconsistencies failed: {}", e)))
        })
    })?;

    if cli.json {
        #[derive(serde::Serialize)]
        struct Output {
            inconsistencies: Vec<libbrat_kb::Inconsistency>,
        }
        output_success(cli, Output { inconsistencies });
    } else {
        print_human(cli, &format!("Found {} inconsistencies", inconsistencies.len()));
        for i in inconsistencies {
            print_human(cli, &format!("  [{:?}] {:?} — {}", i.severity, i.kind, i.description));
        }
    }

    Ok(())
}

fn run_check(cli: &Cli, args: &KbCheckArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;
    ctx.require_initialized()?;
    let repo_root = ctx.repo_root.clone();

    print_human(cli, "Syncing from filesystem...");

    let sync_output = run_kb_thread(move || {
        let kb = libbrat_kb::KbService::open(&repo_root)
            .map_err(|e| BratError::Other(format!("KB open failed: {}", e)))?;
        kb.sync_from_fs()
            .map_err(|e| BratError::Other(format!("Sync failed: {}", e)))
    })?;

    if !cli.json {
        print_human(cli, &format!(
            "Sync complete: {} drifted, {} reconciled, {} errors",
            sync_output.drifted.len(),
            sync_output.reconciled.len(),
            sync_output.errors.len()
        ));
        if !sync_output.drifted.is_empty() {
            for slug in &sync_output.drifted {
                print_human(cli, &format!("  drifted: {}", slug));
            }
        }
        if !sync_output.errors.is_empty() {
            for (slug, err) in &sync_output.errors {
                print_human(cli, &format!("  error on {}: {}", slug, err));
            }
        }
    }

    print_human(cli, "Running consistency check...");

    let repo_root = ctx.repo_root.clone();
    let check = run_kb_thread(move || {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| BratError::Other(format!("Runtime creation failed: {}", e)))?;
        rt.block_on(async {
            let kb = libbrat_kb::KbService::open(&repo_root)
                .map_err(|e| BratError::Other(format!("KB open failed: {}", e)))?;
            kb.run_consistency_check().await
                .map_err(|e| BratError::Other(format!("Consistency check failed: {}", e)))
        })
    })?;

    if cli.json {
        #[derive(serde::Serialize)]
        struct Output {
            sync_strategy: String,
            drifted: Vec<String>,
            reconciled: Vec<String>,
            errors: Vec<(String, String)>,
            score: u8,
            product_arch_coverage: f64,
            arch_product_traceability: f64,
            file_component_mapping: f64,
            test_feature_coverage: f64,
            doc_component_parity: f64,
            inconsistencies: Vec<libbrat_kb::Inconsistency>,
        }
        output_success(cli, Output {
            sync_strategy: sync_output.strategy,
            drifted: sync_output.drifted,
            reconciled: sync_output.reconciled,
            errors: sync_output.errors,
            score: check.score(),
            product_arch_coverage: check.product_arch_coverage,
            arch_product_traceability: check.arch_product_traceability,
            file_component_mapping: check.file_component_mapping,
            test_feature_coverage: check.test_feature_coverage,
            doc_component_parity: check.doc_component_parity,
            inconsistencies: check.inconsistencies.clone(),
        });
    } else {
        print_human(cli, &format!("Consistency score: {}", check.score()));
        if check.inconsistencies.is_empty() {
            print_human(cli, "KB is consistent.");
        } else {
            print_human(cli, &format!("Found {} inconsistencies:", check.inconsistencies.len()));
            for i in &check.inconsistencies {
                print_human(cli, &format!("  [{:?}] {:?} — {}", i.severity, i.kind, i.description));
            }
        }
    }

    // Enforce minimum score threshold if requested
    if let Some(threshold) = args.min_score {
        if check.score() < threshold {
            return Err(BratError::Other(format!(
                "consistency score {} below threshold {}",
                check.score(), threshold
            )));
        }
    }

    Ok(())
}

fn run_edit(cli: &Cli, args: &KbEditArgs) -> Result<(), BratError> {
    let json = cli.json;
    let ctx = BratContext::resolve(cli)?;
    ctx.require_initialized()?;
    let repo_root = ctx.repo_root.clone();
    let slug = args.slug.clone();
    let no_check = args.no_check;

    run_kb_thread(move || {
        let kb = libbrat_kb::KbService::open(&repo_root)
            .map_err(|e| BratError::Other(format!("KB open failed: {}", e)))?;

        let path = kb.ensure_mirror_file(&slug)
            .map_err(|e| BratError::Other(format!("{}", e)))?;

        // Spawn $EDITOR
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| {
            if cfg!(windows) {
                "notepad".to_string()
            } else {
                "vi".to_string()
            }
        });

        let status = std::process::Command::new(&editor)
            .arg(&path)
            .status()
            .map_err(|e| BratError::Io(e))?;

        if !status.success() {
            return Err(BratError::Other(format!("editor exited with code {:?}", status.code())));
        }

        // Sync from filesystem back into SQLite
        let sync_output = kb.sync_from_fs()
            .map_err(|e| BratError::Other(format!("Sync failed: {}", e)))?;

        if !json {
            println!(
                "Sync complete: {} drifted, {} reconciled, {} errors",
                sync_output.drifted.len(),
                sync_output.reconciled.len(),
                sync_output.errors.len()
            );
        }

        if !no_check {
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| BratError::Other(format!("Runtime creation failed: {}", e)))?;
            let check = rt.block_on(async {
                kb.run_consistency_check().await
                    .map_err(|e| BratError::Other(format!("Consistency check failed: {}", e)))
            })?;

            if !json {
                println!("Consistency score: {}", check.score());
                if check.inconsistencies.is_empty() {
                    println!("KB is consistent.");
                } else {
                    println!("Found {} inconsistencies:", check.inconsistencies.len());
                    for i in &check.inconsistencies {
                        println!("  [{:?}] {:?} — {}", i.severity, i.kind, i.description);
                    }
                }
            }
        }

        Ok(())
    })?;

    Ok(())
}
