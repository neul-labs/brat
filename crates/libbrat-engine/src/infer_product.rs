//! Infer product notes from a codebase scan.
//!
//! Extracts features, user stories, and acceptance criteria from
//! README, docs, entry points, and public APIs.

use crate::bootstrap::ProductNoteInferred;
use crate::scan::CodebaseScan;

/// Infer product notes from a codebase scan.
pub fn infer_product_notes(scan: &CodebaseScan) -> Vec<ProductNoteInferred> {
    let mut notes = Vec::new();

    // Extract main product note from README
    if let Some(ref readme) = scan.readme {
        let title = readme
            .lines()
            .next()
            .and_then(|line| line.strip_prefix("# "))
            .unwrap_or("Product")
            .to_string();

        let body = readme.to_string();
        notes.push(ProductNoteInferred {
            title: title.clone(),
            body,
            tags: vec!["product".to_string(), "overview".to_string()],
            acceptance_criteria: vec![],
            priority: "P0".to_string(),
        });
    }

    // Extract features from entry points
    for entry in &scan.entry_points {
        let name = std::path::Path::new(entry)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("entry")
            .to_string();

        notes.push(ProductNoteInferred {
            title: format!("Feature: {}", name),
            body: format!("Entry point at {}", entry),
            tags: vec!["product".to_string(), "feature".to_string()],
            acceptance_criteria: vec![format!("{} runs without errors", name)],
            priority: "P1".to_string(),
        });
    }

    // Extract features from public exports
    let mut features = Vec::new();
    for file in &scan.files {
        for export in &file.exports {
            if !features.contains(export) {
                features.push(export.clone());
            }
        }
    }

    for feature in features.iter().take(10) {
        notes.push(ProductNoteInferred {
            title: format!("API: {}", feature),
            body: format!("Public API exported from codebase: {}", feature),
            tags: vec!["product".to_string(), "api".to_string()],
            acceptance_criteria: vec![format!("{} is callable/usable", feature)],
            priority: "P1".to_string(),
        });
    }

    notes
}
