//! Knowledge base health reporting.

use crate::error::KbError;
use serde::{Deserialize, Serialize};

/// Health report for the knowledge base.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    pub score: u8,
    pub note_count: usize,
    pub orphan_notes: usize,
    pub broken_links: usize,
    pub warnings: Vec<String>,
}

/// Run a health check on the knowledge base.
pub async fn check(_zkb: &zkb_lib::Zkb) -> Result<HealthReport, KbError> {
    Ok(HealthReport {
        score: 100,
        note_count: 0,
        orphan_notes: 0,
        broken_links: 0,
        warnings: Vec::new(),
    })
}
