//! Memory knowledge base operations.
//!
//! Memory notes capture agent discoveries, conventions, and lessons learned.
//!
//! Reads are filesystem-first via the zkb Mirror; falls back to SQLite.

use crate::error::KbError;
use serde::{Deserialize, Serialize};
use zkb_lib::zkb_core::NoteType;
use zkb_lib::zkb_notes::crud::CreateNoteRequest;

/// A memory note from an agent session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryNote {
    pub title: String,
    pub body: String,
    pub component: Option<String>,
    pub task_id: Option<String>,
    pub tags: Vec<String>,
}

/// Summary view of a memory note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryNoteSummary {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub component: Option<String>,
    pub task_id: Option<String>,
}

/// Create a memory note in zkb (writes SQLite + mirror).
pub async fn create(
    zkb: &zkb_lib::Zkb,
    note: &MemoryNote,
) -> Result<String, KbError> {
    let mut tags = vec!["memory".to_string()];
    tags.extend(note.tags.iter().cloned());

    if let Some(ref comp) = note.component {
        tags.push(format!("component:{}", comp));
    }
    if let Some(ref task) = note.task_id {
        tags.push(format!("task:{}", task));
    }

    let request = CreateNoteRequest {
        title: note.title.clone(),
        content: note.body.clone(),
        slug: None,
        note_type: NoteType::Fleeting,
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

/// List memory notes, optionally filtered (filesystem first, fallback to SQLite).
pub async fn list(
    zkb: &zkb_lib::Zkb,
    mirror: Option<&zkb_lib::zkb_storage::Mirror>,
    _component: Option<&str>,
    _task_id: Option<&str>,
) -> Result<Vec<MemoryNoteSummary>, KbError> {
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
                            if fm.tags.contains(&"memory".to_string()) {
                                results.push(MemoryNoteSummary {
                                    id: fm.id,
                                    slug: fm.slug,
                                    title: fm.title,
                                    component: None,
                                    task_id: None,
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
        .list_notes(Some(NoteType::Fleeting), Some("memory"), 1000, 0)
        .map_err(|e| KbError::ZkbOperation(e.to_string()))?;

    Ok(notes
        .into_iter()
        .map(|n| MemoryNoteSummary {
            id: n.id,
            slug: n.slug,
            title: n.title,
            component: None,
            task_id: None,
        })
        .collect())
}
