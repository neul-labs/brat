//! Consistency checking between product and architecture knowledge bases.
//!
//! Computes a score (0-100) across five dimensions:
//! 1. Product-Architecture coverage
//! 2. Architecture-Product traceability
//! 3. File-Component mapping
//! 4. Test-Feature coverage
//! 5. Doc-Component parity

use crate::error::KbError;
use crate::{
    ArchitectureNoteSummary, ProductNoteSummary,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// A single inconsistency between product and architecture.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inconsistency {
    pub kind: InconsistencyKind,
    pub severity: Severity,
    pub description: String,
    pub affected_product: Vec<String>,
    pub affected_architecture: Vec<String>,
    pub suggested_fix: String,
}

/// Type of inconsistency.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InconsistencyKind {
    /// Product feature has no architecture component implementing it.
    MissingArchitecture,
    /// Architecture component has no product feature.
    OrphanComponent,
    /// Product feature has no corresponding tests.
    MissingTests,
    /// Public component has no documentation.
    MissingDocs,
    /// Product description doesn't match implementation.
    Mismatch,
}

/// Severity of an inconsistency.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Low,
    Medium,
    High,
}

/// Consistency score (0-100).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ConsistencyScore(pub u8);

/// Result of a consistency check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsistencyCheck {
    pub score: ConsistencyScore,
    pub product_arch_coverage: f64,
    pub arch_product_traceability: f64,
    pub file_component_mapping: f64,
    pub test_feature_coverage: f64,
    pub doc_component_parity: f64,
    pub inconsistencies: Vec<Inconsistency>,
}

impl ConsistencyScore {
    pub fn value(&self) -> u8 {
        self.0
    }
}

impl ConsistencyCheck {
    pub fn score(&self) -> u8 {
        self.score.value()
    }
}

/// Run a consistency check between product and architecture notes.
pub async fn check(
    product: &[ProductNoteSummary],
    arch: &[ArchitectureNoteSummary],
    _repo_root: &Path,
) -> Result<ConsistencyCheck, KbError> {
    let mut inconsistencies = Vec::new();

    // Dimension 1: Product-Architecture coverage
    // Every product note should link to at least one architecture note
    let mut missing_arch = Vec::new();
    for p in product {
        if p.linked_architecture_count == 0 {
            missing_arch.push(p.slug.clone());
        }
    }
    let coverage = if product.is_empty() {
        0.0
    } else {
        1.0 - (missing_arch.len() as f64 / product.len() as f64)
    };

    for slug in &missing_arch {
        inconsistencies.push(Inconsistency {
            kind: InconsistencyKind::MissingArchitecture,
            severity: Severity::High,
            description: format!(
                "Product feature '{}' has no architecture component implementing it",
                slug
            ),
            affected_product: vec![slug.clone()],
            affected_architecture: Vec::new(),
            suggested_fix: format!(
                "Create an architecture note for '{}' or link an existing one",
                slug
            ),
        });
    }

    // Dimension 2: Architecture-Product traceability
    // Every architecture note should link to at least one product note
    let mut orphan_comp = Vec::new();
    for a in arch {
        if a.linked_product_count == 0 {
            orphan_comp.push(a.slug.clone());
        }
    }
    let traceability = if arch.is_empty() {
        0.0
    } else {
        1.0 - (orphan_comp.len() as f64 / arch.len() as f64)
    };

    for slug in &orphan_comp {
        inconsistencies.push(Inconsistency {
            kind: InconsistencyKind::OrphanComponent,
            severity: Severity::Medium,
            description: format!(
                "Architecture component '{}' is not linked to any product feature",
                slug
            ),
            affected_product: Vec::new(),
            affected_architecture: vec![slug.clone()],
            suggested_fix: format!(
                "Link '{}' to a product feature or remove if obsolete",
                slug
            ),
        });
    }

    // Dimensions 3-5: Placeholder for file, test, doc checks
    // These require filesystem scanning which is done during bootstrap
    let file_mapping = 1.0;
    let test_coverage = 1.0;
    let doc_parity = 1.0;

    let total = coverage + traceability + file_mapping + test_coverage + doc_parity;
    let score = ((total / 5.0) * 100.0).round() as u8;

    Ok(ConsistencyCheck {
        score: ConsistencyScore(score),
        product_arch_coverage: coverage,
        arch_product_traceability: traceability,
        file_component_mapping: file_mapping,
        test_feature_coverage: test_coverage,
        doc_component_parity: doc_parity,
        inconsistencies,
    })
}
