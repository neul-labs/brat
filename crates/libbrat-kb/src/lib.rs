//! Brat Knowledge Base Library
//!
//! This crate provides typed knowledge base operations via direct `zkb-lib`
//! integration. All calls are library-level — no CLI subprocesses.
//!
//! Three axes of knowledge:
//! - **Product**: requirements, user stories, acceptance criteria
//! - **Architecture**: components, ADRs, interfaces, design decisions
//! - **Memory**: agent discoveries, conventions, lessons learned
//!
//! # Example
//!
//! ```no_run
//! use libbrat_kb::KbService;
//! use std::path::Path;
//!
//! #[tokio::main]
//! async fn main() {
//!     let kb = KbService::open(Path::new("/path/to/repo")).unwrap();
//!
//!     // List product notes
//!     let notes = kb.list_product_notes().await.unwrap();
//!     for note in notes {
//!         println!("{}: {}", note.id, note.title);
//!     }
//! }
//! ```

pub mod architecture;
pub mod consistency;
pub mod error;
pub mod health;
pub mod memory;
pub mod product;
pub mod tenant;

pub use architecture::{
    ArchitectureNote, ArchitectureNoteInput, ArchitectureNoteSummary,
};
pub use consistency::{ConsistencyCheck, ConsistencyScore, Inconsistency, InconsistencyKind};
pub use error::KbError;
pub use health::HealthReport;
pub use memory::MemoryNote;
pub use product::{ProductNote, ProductNoteInput, ProductNoteSummary};

use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// Knowledge base service backed by zkb with filesystem as source of truth.
pub struct KbService {
    zkb: std::sync::Arc<zkb_lib::Zkb>,
    tenant: String,
    repo_root: PathBuf,
}

impl KbService {
    /// Open or create a knowledge base for the given repo.
    ///
    /// The zkb vault lives at `<repo_root>/.brat` so the markdown mirror
    /// (`<repo_root>/.brat/notes/*.md`) is inside the repo and git-trackable.
    pub fn open(repo_root: &Path) -> Result<Self, KbError> {
        let tenant = tenant::repo_to_tenant(repo_root);
        let vault_path = repo_root.join(".brat");

        let zkb = if vault_path.join(".zkb").join("zkb.db").exists() {
            info!(tenant = %tenant, path = %vault_path.display(), "opening existing knowledge base");
            zkb_lib::Zkb::open(&vault_path)
                .map_err(|e| KbError::ZkbOpen(e.to_string()))?
        } else {
            info!(tenant = %tenant, path = %vault_path.display(), "initializing new knowledge base");
            zkb_lib::Zkb::init(&vault_path, zkb_lib::zkb_storage::VaultConfig::default())
                .map_err(|e| KbError::ZkbOpen(e.to_string()))?
        };

        Ok(Self {
            zkb: std::sync::Arc::new(zkb),
            tenant,
            repo_root: repo_root.to_path_buf(),
        })
    }

    /// Tenant name used for this repo.
    pub fn tenant(&self) -> &str {
        &self.tenant
    }

    /// Repo root path.
    pub fn repo_root(&self) -> &Path {
        &self.repo_root
    }

    /// Access the underlying zkb handle.
    pub fn zkb(&self) -> &zkb_lib::Zkb {
        &self.zkb
    }

    /// Access the zkb mirror for filesystem-first reads.
    pub fn mirror(&self) -> Option<&zkb_lib::zkb_storage::Mirror> {
        self.zkb.vault().mirror()
    }

    /// Ensure a mirror file exists for the given slug.
    /// If the mirror file is missing, fetches the note from SQLite and writes it.
    pub fn ensure_mirror_file(&self, slug: &str) -> Result<std::path::PathBuf, KbError> {
        let mirror = self.mirror()
            .ok_or_else(|| KbError::ZkbOperation("mirror not available".to_string()))?;
        let path = mirror.path_for(slug);
        if !path.exists() {
            let note = self.zkb.get_note(slug)
                .map_err(|e| KbError::NoteNotFound(e.to_string()))?
                .ok_or_else(|| KbError::NoteNotFound(slug.to_string()))?;

            let frontmatter = zkb_lib::zkb_storage::MirrorFrontmatter {
                id: note.meta.id.to_string(),
                title: note.meta.title.clone(),
                slug: slug.to_string(),
                note_type: note.meta.note_type.to_string(),
                tags: note.tags.clone(),
                created_at: note.meta.created_at.to_rfc3339(),
                updated_at: note.meta.updated_at.to_rfc3339(),
                source_url: note.meta.source_url.clone(),
                source_authors: note.meta.source_authors.clone(),
                folgezettel_id: note.meta.folgezettel_id.clone(),
            };

            mirror.write_note(&frontmatter, &note.content, zkb_lib::zkb_storage::DriftAction::Overwrite)
                .map_err(|e| KbError::ZkbOperation(format!("Write mirror failed: {}", e)))?;
        }
        Ok(path)
    }

    /// Sync hand-edited mirror files back into the SQLite search index.
    pub fn sync_from_fs(&self) -> Result<zkb_lib::actions::SyncMirrorOutput, KbError> {
        let args = zkb_lib::actions::SyncMirrorArgs {
            strategy: Some(zkb_lib::actions::SyncStrategy::Import),
        };
        let output = zkb_lib::actions::sync_mirror(self.zkb.vault(), args)
            .map_err(|e| KbError::ZkbOperation(e.to_string()))?;
        info!(drifted = output.drifted.len(), reconciled = output.reconciled.len(), "synced from filesystem");
        Ok(output)
    }

    // ------------------------------------------------------------------
    // Product notes
    // ------------------------------------------------------------------

    /// Create or update product notes.
    pub async fn upsert_product_notes(
        &self,
        notes: &[ProductNoteInput],
    ) -> Result<Vec<String>, KbError> {
        let mut ids = Vec::new();
        for note in notes {
            debug!(title = %note.title, "upserting product note");
            let id = product::upsert(&self.zkb, note).await?;
            ids.push(id);
        }
        info!(count = ids.len(), "upserted product notes");
        Ok(ids)
    }

    /// List all product notes (reads from filesystem first).
    pub async fn list_product_notes(&self) -> Result<Vec<ProductNoteSummary>, KbError> {
        product::list(&self.zkb, self.mirror()).await
    }

    /// Get a product note by slug (reads from filesystem first).
    pub async fn get_product_note(&self, slug: &str) -> Result<ProductNote, KbError> {
        product::get(&self.zkb, self.mirror(), slug).await
    }

    // ------------------------------------------------------------------
    // Architecture notes
    // ------------------------------------------------------------------

    /// Create or update architecture notes.
    pub async fn upsert_architecture_notes(
        &self,
        notes: &[ArchitectureNoteInput],
    ) -> Result<Vec<String>, KbError> {
        let mut ids = Vec::new();
        for note in notes {
            debug!(title = %note.title, "upserting architecture note");
            let id = architecture::upsert(&self.zkb, note).await?;
            ids.push(id);
        }
        info!(count = ids.len(), "upserted architecture notes");
        Ok(ids)
    }

    /// List all architecture notes (reads from filesystem first).
    pub async fn list_architecture_notes(&self) -> Result<Vec<ArchitectureNoteSummary>, KbError> {
        architecture::list(&self.zkb, self.mirror()).await
    }

    /// Get an architecture note by slug (reads from filesystem first).
    pub async fn get_architecture_note(&self, slug: &str) -> Result<ArchitectureNote, KbError> {
        architecture::get(&self.zkb, self.mirror(), slug).await
    }

    // ------------------------------------------------------------------
    // Memory notes
    // ------------------------------------------------------------------

    /// Create a memory note from agent session findings.
    pub async fn create_memory_note(
        &self,
        note: &MemoryNote,
    ) -> Result<String, KbError> {
        debug!(title = %note.title, "creating memory note");
        memory::create(&self.zkb, note).await
    }

    /// List memory notes, optionally filtered by component or task.
    pub async fn list_memory_notes(
        &self,
        component: Option<&str>,
        task_id: Option<&str>,
    ) -> Result<Vec<memory::MemoryNoteSummary>, KbError> {
        memory::list(&self.zkb, self.mirror(), component, task_id).await
    }

    // ------------------------------------------------------------------
    // Consistency
    // ------------------------------------------------------------------

    /// Compute overall consistency score (0-100).
    pub async fn get_consistency_score(&self) -> Result<u8, KbError> {
        let check = self.run_consistency_check().await?;
        Ok(check.score())
    }

    /// Run full consistency check between product and architecture.
    pub async fn run_consistency_check(&self) -> Result<ConsistencyCheck, KbError> {
        let product = self.list_product_notes().await?;
        let arch = self.list_architecture_notes().await?;
        consistency::check(&product, &arch, &self.repo_root).await
    }

    /// List all inconsistencies.
    pub async fn list_inconsistencies(&self) -> Result<Vec<Inconsistency>, KbError> {
        let check = self.run_consistency_check().await?;
        Ok(check.inconsistencies)
    }

    // ------------------------------------------------------------------
    // Health
    // ------------------------------------------------------------------

    /// Run knowledge base health check.
    pub async fn health(&self) -> Result<HealthReport, KbError> {
        health::check(&self.zkb).await
    }

    // ------------------------------------------------------------------
    // Search
    // ------------------------------------------------------------------

    /// Full-text search across all note types (uses SQLite index).
    pub async fn search(&self, query: &str) -> Result<Vec<SearchResult>, KbError> {
        let results = self.zkb
            .search(query, 50)
            .map_err(|e| KbError::ZkbSearch(e.to_string()))?;

        Ok(results
            .results
            .into_iter()
            .map(|r| SearchResult {
                slug: r.slug,
                title: r.title,
                note_type: r.tags.first().cloned().unwrap_or_default(),
                score: r.score,
            })
            .collect())
    }
}

/// A search result from the knowledge base.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchResult {
    pub slug: String,
    pub title: String,
    pub note_type: String,
    pub score: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn filesystem_first_reads_and_sync() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().join("repo");
        std::fs::create_dir(&repo).unwrap();
        std::fs::create_dir(repo.join(".brat")).unwrap();

        // Open (init) vault
        let kb = KbService::open(&repo).unwrap();

        // Create a product note
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            kb.upsert_product_notes(&[ProductNoteInput {
                    title: "Test Feature".to_string(),
                    body: "This is a test feature.".to_string(),
                    tags: vec!["test".to_string()],
                    acceptance_criteria: vec!["It works".to_string()],
                    priority: Some("P1".to_string()),
                }],
            ).await.unwrap();
        });

        // Verify mirror file exists
        let notes_dir = repo.join(".brat").join("notes");
        let mirror_files: Vec<_> = std::fs::read_dir(&notes_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("md"))
            .collect();
        assert!(!mirror_files.is_empty(), "mirror file should exist");

        // Hand-edit the mirror file
        let mirror_path = &mirror_files[0].path();
        let original = std::fs::read_to_string(mirror_path).unwrap();
        let edited = original.replace("This is a test feature.", "This is a hand-edited test feature.");
        let mut f = std::fs::File::create(mirror_path).unwrap();
        f.write_all(edited.as_bytes()).unwrap();
        drop(f);

        // Verify filesystem read sees the edit
        let rt = tokio::runtime::Runtime::new().unwrap();
        let notes = rt.block_on(async {
            kb.list_product_notes().await.unwrap()
        });
        let note = notes.into_iter().find(|n| n.title == "Test Feature").unwrap();
        assert_eq!(note.title, "Test Feature");

        // Verify get_product_note sees the edit
        let rt = tokio::runtime::Runtime::new().unwrap();
        let full = rt.block_on(async {
            kb.get_product_note(&note.slug).await.unwrap()
        });
        assert!(full.body.contains("hand-edited"), "filesystem read should reflect hand-edit: got {}", full.body);

        // Sync from filesystem to SQLite
        let output = kb.sync_from_fs().unwrap();
        assert_eq!(output.strategy, "import");
        assert!(!output.drifted.is_empty(), "drift should be detected after hand-edit");
        assert!(!output.reconciled.is_empty(), "hand-edited note should be reconciled");
        assert!(output.errors.is_empty(), "no errors during sync");

        // Verify SQLite search sees the edit after sync
        let rt = tokio::runtime::Runtime::new().unwrap();
        let results = rt.block_on(async {
            kb.search("hand edited").await.unwrap()
        });
        assert!(
            results.iter().any(|r| r.title == "Test Feature"),
            "search should find hand-edited note after sync"
        );
    }
}
