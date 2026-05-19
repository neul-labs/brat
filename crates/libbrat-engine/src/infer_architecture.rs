//! Infer architecture notes from a codebase scan and product notes.
//!
//! Extracts components, interfaces, and design decisions from
//! module structure, public APIs, and trait definitions.

use crate::bootstrap::ArchitectureNoteInferred;
use crate::scan::CodebaseScan;
use std::collections::HashMap;

/// Infer architecture notes from a codebase scan and product notes.
pub fn infer_architecture_notes(
    scan: &CodebaseScan,
    _product: &[crate::bootstrap::ProductNoteInferred],
) -> Vec<ArchitectureNoteInferred> {
    let mut notes = Vec::new();

    // Group files by directory = component
    let mut components: HashMap<String, Vec<String>> = HashMap::new();
    for file in &scan.files {
        if let Some(parent) = file.path.parent() {
            let comp = parent
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("root")
                .to_string();
            components
                .entry(comp)
                .or_default()
                .push(file.path.to_string_lossy().to_string());
        }
    }

    // Create architecture note per component
    for (comp_name, paths) in &components {
        if paths.is_empty() {
            continue;
        }

        let mut interfaces = Vec::new();
        let mut functions = Vec::new();

        for file in scan.files.iter().filter(|f| {
            f.path.parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                == Some(comp_name)
        }) {
            interfaces.extend(file.exports.clone());
            functions.extend(file.functions.clone());
        }

        notes.push(ArchitectureNoteInferred {
            title: format!("Component: {}", comp_name),
            body: format!(
                "Component '{}' contains {} files.\n\nPublic interfaces: {}\n\nKey functions: {}",
                comp_name,
                paths.len(),
                interfaces.join(", "),
                functions.join(", ")
            ),
            tags: vec!["architecture".to_string(), "component".to_string()],
            components: vec![comp_name.clone()],
            interfaces,
            file_paths: paths.clone(),
        });
    }

    // Overall architecture note
    let languages: Vec<String> = scan
        .language_stats
        .iter()
        .map(|(lang, count)| format!("{} ({} files)", lang, count))
        .collect();

    notes.push(ArchitectureNoteInferred {
        title: "System Architecture".to_string(),
        body: format!(
            "Languages: {}\n\nComponents: {}\n\nEntry Points: {}\n\nDependencies: {}",
            languages.join(", "),
            components.len(),
            scan.entry_points.join(", "),
            scan.dependencies.join(", ")
        ),
        tags: vec!["architecture".to_string(), "system".to_string()],
        components: components.keys().cloned().collect(),
        interfaces: Vec::new(),
        file_paths: Vec::new(),
    });

    notes
}
