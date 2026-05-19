//! Product knowledge base operations.
//!
//! Product notes capture requirements, user stories, acceptance criteria,
//! and feature descriptions.
//!
//! Reads are filesystem-first via the zkb Mirror; falls back to SQLite.

use crate::error::KbError;
use serde::{Deserialize, Serialize};
use zkb_lib::zkb_core::NoteType;
use zkb_lib::zkb_notes::crud::CreateNoteRequest;

/// Input for creating or updating a product note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductNoteInput {
    pub title: String,
    pub body: String,
    pub tags: Vec<String>,
    pub acceptance_criteria: Vec<String>,
    pub priority: Option<String>,
}

/// A product note as stored in the knowledge base.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductNote {
    pub slug: String,
    pub title: String,
    pub body: String,
    pub tags: Vec<String>,
    pub acceptance_criteria: Vec<String>,
    pub priority: String,
    pub created_at: String,
    pub updated_at: String,
    pub linked_architecture: Vec<String>,
}

/// Summary view of a product note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductNoteSummary {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub priority: String,
    pub linked_architecture_count: usize,
}

/// Upsert a product note into zkb (writes SQLite + mirror).
pub async fn upsert(
    zkb: &zkb_lib::Zkb,
    input: &ProductNoteInput,
) -> Result<String, KbError> {
    let tags = input
        .tags
        .iter()
        .cloned()
        .chain(std::iter::once("product".to_string()))
        .collect::<Vec<_>>();

    let body = format!(
        "{body}\n\n## Acceptance Criteria\n{ac}\n\n## Priority\n{prio}",
        body = input.body,
        ac = input.acceptance_criteria.join("\n- "),
        prio = input.priority.as_deref().unwrap_or("P1"),
    );

    let request = CreateNoteRequest {
        title: input.title.clone(),
        content: body,
        slug: None,
        note_type: NoteType::Permanent,
        tags,
        source_path: None,
        folgezettel_parent: None,
        source_url: None,
        source_authors: vec![],
    };

    let result = zkb
        .create_note_with(request)
        .map_err(|e| KbError::ZkbOperation(e.to_string()))?;

    Ok(result.meta.slug.to_string())
}

/// List all product notes (filesystem first, fallback to SQLite).
pub async fn list(
    zkb: &zkb_lib::Zkb,
    mirror: Option<&zkb_lib::zkb_storage::Mirror>,
) -> Result<Vec<ProductNoteSummary>, KbError> {
    // Filesystem-first path
    if let Some(m) = mirror {
        let notes_dir = m.notes_dir();
        let mut results = Vec::new();
        if let Ok(entries) = std::fs::read_dir(notes_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) != Some("md") {
                    continue;
                }
                let slug = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();
                if slug.is_empty() {
                    continue;
                }
                match m.read_file(&slug) {
                    Ok(Some((yaml, _body))) => {
                        if let Ok(fm) = serde_yaml::from_str::<zkb_lib::zkb_storage::MirrorFrontmatter>(&yaml) {
                            if fm.tags.contains(&"product".to_string()) {
                                results.push(ProductNoteSummary {
                                    id: fm.id,
                                    slug: fm.slug,
                                    title: fm.title,
                                    priority: "P1".to_string(),
                                    linked_architecture_count: 0,
                                });
                            }
                        }
                    }
                    Ok(None) => continue,
                    Err(_) => continue,
                }
            }
        }
        if !results.is_empty() {
            return Ok(results);
        }
    }

    // Fallback to SQLite
    let notes = zkb
        .list_notes(Some(NoteType::Permanent), Some("product"), 1000, 0)
        .map_err(|e| KbError::ZkbOperation(e.to_string()))?;

    Ok(notes
        .into_iter()
        .map(|n| ProductNoteSummary {
            id: n.id,
            slug: n.slug,
            title: n.title,
            priority: "P1".to_string(),
            linked_architecture_count: 0,
        })
        .collect())
}

/// Get a product note by slug (filesystem first, fallback to SQLite).
pub async fn get(
    zkb: &zkb_lib::Zkb,
    mirror: Option<&zkb_lib::zkb_storage::Mirror>,
    slug: &str,
) -> Result<ProductNote, KbError> {
    // Filesystem-first path
    if let Some(m) = mirror {
        if let Ok(Some((yaml, body))) = m.read_file(slug) {
            if let Ok(fm) = serde_yaml::from_str::<zkb_lib::zkb_storage::MirrorFrontmatter>(&yaml) {
                return Ok(ProductNote {
                    slug: fm.slug,
                    title: fm.title,
                    body,
                    tags: fm.tags,
                    acceptance_criteria: Vec::new(),
                    priority: "P1".to_string(),
                    created_at: fm.created_at,
                    updated_at: fm.updated_at,
                    linked_architecture: Vec::new(),
                });
            }
        }
    }

    // Fallback to SQLite
    let note = zkb
        .get_note(slug)
        .map_err(|e| KbError::NoteNotFound(e.to_string()))?;

    let note = note.ok_or_else(|| KbError::NoteNotFound(slug.to_string()))?;

    Ok(ProductNote {
        slug: note.meta.slug.to_string(),
        title: note.meta.title,
        body: note.content,
        tags: note.tags,
        acceptance_criteria: Vec::new(),
        priority: "P1".to_string(),
        created_at: note.meta.created_at.to_rfc3339(),
        updated_at: note.meta.updated_at.to_rfc3339(),
        linked_architecture: Vec::new(),
    })
}
