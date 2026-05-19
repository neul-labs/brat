//! Consistency checking between product and architecture for the bootstrap process.
//!
//! This is a simplified version of the libbrat-kb consistency check
//! that works with inferred notes during bootstrap.

use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::bootstrap::{ArchitectureNoteInferred, ProductNoteInferred};

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
    MissingArchitecture,
    OrphanComponent,
    MissingTests,
    MissingDocs,
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

impl ConsistencyCheck {
    pub fn score(&self) -> u8 {
        self.score.0
    }
}

/// Check consistency between inferred product and architecture notes.
pub async fn check(
    product: &[ProductNoteInferred],
    arch: &[ArchitectureNoteInferred],
    _repo_root: &Path,
) -> Result<ConsistencyCheck, String> {
    let mut inconsistencies = Vec::new();

    // Dimension 1: Product-Architecture coverage
    let product_titles: Vec<String> = product.iter().map(|p| p.title.clone()).collect();
    let arch_components: Vec<String> = arch
        .iter()
        .flat_map(|a| a.components.clone())
        .collect();

    let mut missing_arch = Vec::new();
    for prod in product {
        let has_arch = arch.iter().any(|a| {
            a.body.contains(&prod.title)
                || prod.title.split(' ').any(|word| a.components.contains(&word.to_lowercase()))
        });
        if !has_arch {
            missing_arch.push(prod.title.clone());
        }
    }

    let coverage = if product.is_empty() {
        0.0
    } else {
        1.0 - (missing_arch.len() as f64 / product.len() as f64)
    };

    for title in &missing_arch {
        inconsistencies.push(Inconsistency {
            kind: InconsistencyKind::MissingArchitecture,
            severity: Severity::High,
            description: format!(
                "Product feature '{}' has no architecture component implementing it",
                title
            ),
            affected_product: vec![title.clone()],
            affected_architecture: Vec::new(),
            suggested_fix: format!(
                "Create an architecture note for '{}' or link an existing one",
                title
            ),
        });
    }

    // Dimension 2: Architecture-Product traceability
    let mut orphan_comp = Vec::new();
    for comp in &arch_components {
        let has_product = product_titles.iter().any(|p| {
            p.to_lowercase().contains(&comp.to_lowercase())
                || comp.to_lowercase().contains(&p.to_lowercase().split(' ').next().unwrap_or(""))
        });
        if !has_product {
            orphan_comp.push(comp.clone());
        }
    }

    let traceability = if arch_components.is_empty() {
        0.0
    } else {
        1.0 - (orphan_comp.len() as f64 / arch_components.len() as f64)
    };

    for comp in &orphan_comp {
        inconsistencies.push(Inconsistency {
            kind: InconsistencyKind::OrphanComponent,
            severity: Severity::Medium,
            description: format!(
                "Architecture component '{}' is not linked to any product feature",
                comp
            ),
            affected_product: Vec::new(),
            affected_architecture: vec![comp.clone()],
            suggested_fix: format!(
                "Link '{}' to a product feature or remove if obsolete",
                comp
            ),
        });
    }

    // Dimensions 3-5: Placeholder (need file scanning)
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
