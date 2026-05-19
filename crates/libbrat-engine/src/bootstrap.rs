//! Bootstrap algorithm for the software factory.
//!
//! Auto-generates product and architecture KB notes from an existing codebase,
//! then iterates until consistency is reached or max iterations exhausted.

use crate::consistency::Inconsistency;
use serde::{Deserialize, Serialize};

/// Result of the bootstrap process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapResult {
    /// Whether product and architecture notes are fully consistent.
    pub consistent: bool,
    /// Number of iterations run.
    pub iterations: u32,
    /// List of remaining inconsistencies (empty if consistent).
    pub inconsistencies: Vec<Inconsistency>,
    /// Inferred product notes.
    pub product_notes: Vec<ProductNoteInferred>,
    /// Inferred architecture notes.
    pub arch_notes: Vec<ArchitectureNoteInferred>,
}

/// A product note inferred from the codebase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductNoteInferred {
    pub title: String,
    pub body: String,
    pub tags: Vec<String>,
    pub acceptance_criteria: Vec<String>,
    pub priority: String,
}

/// An architecture note inferred from the codebase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureNoteInferred {
    pub title: String,
    pub body: String,
    pub tags: Vec<String>,
    pub components: Vec<String>,
    pub interfaces: Vec<String>,
    pub file_paths: Vec<String>,
}

/// Try to auto-fix inconsistencies.
///
/// Returns true if any fixes were applied.
pub fn auto_fix(inconsistencies: &mut Vec<Inconsistency>) -> bool {
    let before = inconsistencies.len();

    // Remove auto-fixable inconsistencies in-place
    inconsistencies.retain(|inc| {
        match inc.kind {
            crate::consistency::InconsistencyKind::MissingArchitecture => {
                // Can't auto-fix: needs human judgment on architecture
                true
            }
            crate::consistency::InconsistencyKind::OrphanComponent => {
                // Can't auto-fix: needs human judgment on product
                true
            }
            crate::consistency::InconsistencyKind::MissingTests => {
                // Can't auto-fix: needs human to write tests
                true
            }
            crate::consistency::InconsistencyKind::MissingDocs => {
                // Can't auto-fix: needs human to write docs
                true
            }
            crate::consistency::InconsistencyKind::Mismatch => {
                // Can't auto-fix: needs human judgment
                true
            }
        }
    });

    let after = inconsistencies.len();
    before != after
}
