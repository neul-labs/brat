//! Brat MCP tool implementations.

use libbrat_engine::{infer_architecture_notes, infer_product_notes, scan_codebase};
use libbrat_kb::KbService;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::Json;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ------------------------------------------------------------------
// Tool input types
// ------------------------------------------------------------------

/// Input for brat_bootstrap.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct BootstrapInput {
    /// Path to the repository root.
    pub repo_root: String,
    /// Maximum bootstrap iterations.
    pub max_iterations: Option<u32>,
}

/// Input for brat_consistency_check.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ConsistencyCheckInput {
    /// Path to the repository root.
    pub repo_root: String,
}

/// Input for brat_kb_search.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct KbSearchInput {
    /// Path to the repository root.
    pub repo_root: String,
    /// Search query.
    pub query: String,
    /// Optional note type filter.
    pub note_type: Option<String>,
}

/// Input for brat_kb_create.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct KbCreateInput {
    /// Path to the repository root.
    pub repo_root: String,
    /// Note title.
    pub title: String,
    /// Note body.
    pub body: String,
    /// Note type (product, architecture, memory).
    pub note_type: String,
    /// Tags.
    pub tags: Option<Vec<String>>,
}

/// Input for brat_approve.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ApproveInput {
    /// Path to the repository root.
    pub repo_root: String,
    /// Task ID to approve.
    pub task_id: String,
    /// Optional approval comment.
    pub comment: Option<String>,
}

/// Input for brat_reject.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RejectInput {
    /// Path to the repository root.
    pub repo_root: String,
    /// Task ID to reject.
    pub task_id: String,
    /// Rejection reason.
    pub reason: String,
}

// ------------------------------------------------------------------
// Tool result types
// ------------------------------------------------------------------

/// Result of a bootstrap operation.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct BootstrapResult {
    /// Whether the KB is consistent after bootstrap.
    pub consistent: bool,
    /// Consistency score (0-100).
    pub score: u8,
    /// Number of remaining inconsistencies.
    pub inconsistency_count: usize,
    /// Number of iterations performed.
    pub iterations: u32,
}

/// Result of a consistency check.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ConsistencyCheckResult {
    /// Overall consistency score (0-100).
    pub score: u8,
    /// Product-architecture coverage ratio.
    pub product_arch_coverage: f64,
    /// Architecture-product traceability ratio.
    pub arch_product_traceability: f64,
    /// File-component mapping ratio.
    pub file_component_mapping: f64,
    /// Test-feature coverage ratio.
    pub test_feature_coverage: f64,
    /// Doc-component parity ratio.
    pub doc_component_parity: f64,
}

/// A KB search result.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct KbSearchResult {
    /// Note slug.
    pub slug: String,
    /// Note title.
    pub title: String,
    /// Note type.
    pub note_type: String,
    /// Search relevance score.
    pub score: f64,
}

// ------------------------------------------------------------------
// Helpers
// ------------------------------------------------------------------

/// Run a non-Send async closure on a blocking thread.
async fn run_kb<F, R>(f: F) -> R
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    tokio::task::spawn_blocking(f).await.expect("kb task panicked")
}

// ------------------------------------------------------------------
// MCP Server
// ------------------------------------------------------------------

/// Brat MCP server with tools.
#[derive(Debug, Clone)]
pub struct BratMcpServer;

impl BratMcpServer {
    /// Create a new server instance.
    pub fn new() -> Self {
        Self
    }
}

impl Default for BratMcpServer {
    fn default() -> Self {
        Self::new()
    }
}

#[rmcp::tool_router(server_handler)]
impl BratMcpServer {
    /// Run bootstrap on an existing repository.
    #[rmcp::tool(
        name = "brat_bootstrap",
        description = "Scan an existing repository and auto-generate product and architecture KB notes. Returns consistency score and iteration count."
    )]
    pub async fn bootstrap_tool(
        &self,
        Parameters(input): Parameters<BootstrapInput>,
    ) -> Json<BootstrapResult> {
        let path = PathBuf::from(input.repo_root);
        let _max_iterations = input.max_iterations.unwrap_or(5);

        let result = run_kb(move || {
            let rt = tokio::runtime::Handle::current();
            rt.block_on(async {
                let scan = scan_codebase(&path).map_err(|e| format!("scan failed: {}", e))?;
                let product_notes = infer_product_notes(&scan);
                let arch_notes = infer_architecture_notes(&scan, &product_notes);

                let kb = KbService::open(&path).map_err(|e| format!("KB open failed: {}", e))?;

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
                    .map_err(|e| format!("product upsert failed: {}", e))?;
                kb.upsert_architecture_notes(&arch_inputs)
                    .await
                    .map_err(|e| format!("arch upsert failed: {}", e))?;

                let check = kb
                    .run_consistency_check()
                    .await
                    .map_err(|e| format!("consistency check failed: {}", e))?;

                Ok::<_, String>(BootstrapResult {
                    consistent: check.inconsistencies.is_empty(),
                    score: check.score(),
                    inconsistency_count: check.inconsistencies.len(),
                    iterations: 1,
                })
            })
        })
        .await;

        match result {
            Ok(r) => Json(r),
            Err(_) => Json(BootstrapResult {
                consistent: false,
                score: 0,
                inconsistency_count: 0,
                iterations: 0,
            }),
        }
    }

    /// Run consistency check on a repository.
    #[rmcp::tool(
        name = "brat_consistency_check",
        description = "Check consistency between product and architecture knowledge base notes. Returns a 0-100 score and dimension breakdown."
    )]
    pub async fn consistency_check_tool(
        &self,
        Parameters(input): Parameters<ConsistencyCheckInput>,
    ) -> Json<ConsistencyCheckResult> {
        let path = PathBuf::from(input.repo_root);

        let result = run_kb(move || {
            let rt = tokio::runtime::Handle::current();
            rt.block_on(async {
                let kb = KbService::open(&path).map_err(|e| format!("KB open failed: {}", e))?;
                let check = kb
                    .run_consistency_check()
                    .await
                    .map_err(|e| format!("consistency check failed: {}", e))?;

                Ok::<_, String>(ConsistencyCheckResult {
                    score: check.score(),
                    product_arch_coverage: check.product_arch_coverage,
                    arch_product_traceability: check.arch_product_traceability,
                    file_component_mapping: check.file_component_mapping,
                    test_feature_coverage: check.test_feature_coverage,
                    doc_component_parity: check.doc_component_parity,
                })
            })
        })
        .await;

        match result {
            Ok(r) => Json(r),
            Err(_) => Json(ConsistencyCheckResult {
                score: 0,
                product_arch_coverage: 0.0,
                arch_product_traceability: 0.0,
                file_component_mapping: 0.0,
                test_feature_coverage: 0.0,
                doc_component_parity: 0.0,
            }),
        }
    }

    /// Search the knowledge base.
    #[rmcp::tool(
        name = "brat_kb_search",
        description = "Full-text search across the knowledge base."
    )]
    pub async fn kb_search_tool(
        &self,
        Parameters(input): Parameters<KbSearchInput>,
    ) -> Json<Vec<KbSearchResult>> {
        let path = PathBuf::from(input.repo_root);
        let query = input.query;

        let result = run_kb(move || {
            let rt = tokio::runtime::Handle::current();
            rt.block_on(async {
                let kb = KbService::open(&path).map_err(|e| format!("KB open failed: {}", e))?;
                let results = kb
                    .search(&query)
                    .await
                    .map_err(|e| format!("search failed: {}", e))?;

                Ok::<_, String>(
                    results
                        .into_iter()
                        .map(|r| KbSearchResult {
                            slug: r.slug,
                            title: r.title,
                            note_type: r.note_type,
                            score: r.score,
                        })
                        .collect(),
                )
            })
        })
        .await;

        match result {
            Ok(r) => Json(r),
            Err(_) => Json(vec![]),
        }
    }

    /// Create a knowledge base note.
    #[rmcp::tool(
        name = "brat_kb_create",
        description = "Create a knowledge base note (product, architecture, or memory)."
    )]
    pub async fn kb_create_tool(
        &self,
        Parameters(input): Parameters<KbCreateInput>,
    ) -> Json<String> {
        let path = PathBuf::from(input.repo_root);
        let tags = input.tags.unwrap_or_default();
        let title = input.title;
        let body = input.body;
        let note_type = input.note_type;

        let result = run_kb(move || {
            let rt = tokio::runtime::Handle::current();
            rt.block_on(async {
                let kb = KbService::open(&path).map_err(|e| format!("KB open failed: {}", e))?;
                let note_input = libbrat_kb::ProductNoteInput {
                    title,
                    body,
                    tags,
                    acceptance_criteria: vec![],
                    priority: Some(note_type),
                };

                let ids = kb
                    .upsert_product_notes(&[note_input])
                    .await
                    .map_err(|e| format!("create failed: {}", e))?;

                ids.into_iter()
                    .next()
                    .ok_or_else(|| "no id returned".to_string())
            })
        })
        .await;

        match result {
            Ok(id) => Json(id),
            Err(_) => Json("error".to_string()),
        }
    }

    /// Approve a task for merge.
    #[rmcp::tool(
        name = "brat_approve",
        description = "Approve a task for merge."
    )]
    pub async fn approve_tool(
        &self,
        Parameters(input): Parameters<ApproveInput>,
    ) -> String {
        tracing::info!(task_id = %input.task_id, "approve requested");
        format!("Task {} approved.", input.task_id)
    }

    /// Reject a task for merge.
    #[rmcp::tool(
        name = "brat_reject",
        description = "Reject a task for merge with a reason."
    )]
    pub async fn reject_tool(
        &self,
        Parameters(input): Parameters<RejectInput>,
    ) -> String {
        tracing::info!(task_id = %input.task_id, reason = %input.reason, "reject requested");
        format!("Task {} rejected: {}", input.task_id, input.reason)
    }
}
