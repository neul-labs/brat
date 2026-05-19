//! Architecture knowledge base operations.
//!
//! Architecture notes capture components, ADRs, interfaces, and design decisions.
//!
//! Reads are filesystem-first via the zkb Mirror; falls back to SQLite.

use crate::error::KbError;
use serde::{Deserialize, Serialize};
use zkb_lib::zkb_core::NoteType;
use zkb_lib::zkb_notes::crud::CreateNoteRequest;

/// Input for creating or updating an architecture note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureNoteInput {
    pub title: String,
    pub body: String,
    pub tags: Vec<String>,
    pub components: Vec<String>,
    pub interfaces: Vec<String>,
    pub file_paths: Vec<String>,
}

/// An architecture note as stored in the knowledge base.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureNote {
    pub slug: String,
    pub title: String,
    pub body: String,
    pub tags: Vec<String>,
    pub components: Vec<String>,
    pub interfaces: Vec<String>,
    pub file_paths: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub linked_product: Vec<String>,
}

/// Summary view of an architecture note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureNoteSummary {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub component_count: usize,
    pub linked_product_count: usize,
}

/// Upsert an architecture note into zkb (writes SQLite + mirror).
pub async fn upsert(
    zkb: &zkb_lib::Zkb,
    input: &ArchitectureNoteInput,
) -> Result<String, KbError> {
    let tags = input
        .tags
        .iter()
        .cloned()
        .chain(std::iter::once("architecture".to_string()))
        .collect::<Vec<_>>();

    let body = format!(
        "{body}\n\n## Components\n{comp}\n\n## Interfaces\n{iface}\n\n## File Paths\n{paths}",
        body = input.body,
        comp = input.components.join("\n- "),
        iface = input.interfaces.join("\n- "),
        paths = input.file_paths.join("\n- "),
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

/// List all architecture notes (filesystem first, fallback to SQLite).
pub async fn list(
    zkb: &zkb_lib::Zkb,
    mirror: Option<&zkb_lib::zkb_storage::Mirror>,
) -> Result<Vec<ArchitectureNoteSummary>, KbError> {
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
                            if fm.tags.contains(&"architecture".to_string()) {
                                results.push(ArchitectureNoteSummary {
                                    id: fm.id,
                                    slug: fm.slug,
                                    title: fm.title,
                                    component_count: 0,
                                    linked_product_count: 0,
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
        .list_notes(Some(NoteType::Permanent), Some("architecture"), 1000, 0)
        .map_err(|e| KbError::ZkbOperation(e.to_string()))?;

    Ok(notes
        .into_iter()
        .map(|n| ArchitectureNoteSummary {
            id: n.id,
            slug: n.slug,
            title: n.title,
            component_count: 0,
            linked_product_count: 0,
        })
        .collect())
}

/// Get an architecture note by slug (filesystem first, fallback to SQLite).
pub async fn get(
    zkb: &zkb_lib::Zkb,
    mirror: Option<&zkb_lib::zkb_storage::Mirror>,
    slug: &str,
) -> Result<ArchitectureNote, KbError> {
    // Filesystem-first path
    if let Some(m) = mirror {
        if let Ok(Some((yaml, body))) = m.read_file(slug) {
            if let Ok(fm) = serde_yaml::from_str::<zkb_lib::zkb_storage::MirrorFrontmatter>(&yaml) {
                return Ok(ArchitectureNote {
                    slug: fm.slug,
                    title: fm.title,
                    body,
                    tags: fm.tags,
                    components: Vec::new(),
                    interfaces: Vec::new(),
                    file_paths: Vec::new(),
                    created_at: fm.created_at,
                    updated_at: fm.updated_at,
                    linked_product: Vec::new(),
                });
            }
        }
    }

    // Fallback to SQLite
    let note = zkb
        .get_note(slug)
        .map_err(|e| KbError::NoteNotFound(e.to_string()))?;

    let note = note.ok_or_else(|| KbError::NoteNotFound(slug.to_string()))?;

    Ok(ArchitectureNote {
        slug: note.meta.slug.to_string(),
        title: note.meta.title,
        body: note.content,
        tags: note.tags,
        components: Vec::new(),
        interfaces: Vec::new(),
        file_paths: Vec::new(),
        created_at: note.meta.created_at.to_rfc3339(),
        updated_at: note.meta.updated_at.to_rfc3339(),
        linked_product: Vec::new(),
    })
}
