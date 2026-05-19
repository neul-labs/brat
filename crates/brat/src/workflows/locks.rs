//! Lock management helpers for workflows.
//!
//! This module provides utilities for acquiring and releasing locks
//! in a policy-aware manner.

use std::sync::Arc;

use libbrat_grite::GriteeClient;

use super::error::WorkflowError;

/// Lock policy controlling how lock conflicts are handled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockPolicy {
    /// No lock enforcement - skip lock acquisition entirely.
    Off,
    /// Warn on lock conflicts but continue.
    Warn,
    /// Block operations if lock cannot be acquired.
    Require,
}

impl LockPolicy {
    /// Parse lock policy from string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "off" => Some(LockPolicy::Off),
            "warn" => Some(LockPolicy::Warn),
            "require" => Some(LockPolicy::Require),
            _ => None,
        }
    }
}

impl Default for LockPolicy {
    fn default() -> Self {
        LockPolicy::Warn
    }
}

/// Helper for policy-aware lock acquisition and release.
pub struct LockHelper {
    gritee: Arc<GriteeClient>,
    policy: LockPolicy,
}

impl LockHelper {
    /// Create a new LockHelper.
    pub fn new(gritee: Arc<GriteeClient>, policy: LockPolicy) -> Self {
        Self { gritee, policy }
    }

    /// Create a LockHelper from config policy string.
    pub fn from_config(gritee: Arc<GriteeClient>, policy_str: &str) -> Self {
        let policy = LockPolicy::from_str(policy_str).unwrap_or_default();
        Self::new(gritee, policy)
    }

    /// Get the current lock policy.
    pub fn policy(&self) -> LockPolicy {
        self.policy
    }

    /// Acquire locks for a list of resources.
    ///
    /// Behavior depends on the lock policy:
    /// - `Off`: Skip acquisition, return empty list
    /// - `Warn`: Try to acquire, log warnings on failure, return what was acquired
    /// - `Require`: Try to acquire all, rollback and error on any failure
    ///
    /// Returns the list of successfully acquired resources.
    pub fn acquire_locks(
        &self,
        resources: &[String],
        ttl_ms: i64,
    ) -> Result<Vec<String>, WorkflowError> {
        // Skip entirely if policy is off
        if self.policy == LockPolicy::Off {
            return Ok(vec![]);
        }

        // Skip if no resources to lock
        if resources.is_empty() {
            return Ok(vec![]);
        }

        let mut acquired = Vec::new();

        for resource in resources {
            match self.gritee.lock_acquire(resource, ttl_ms) {
                Ok(result) if result.acquired => {
                    acquired.push(resource.clone());
                }
                Ok(result) => {
                    // Lock held by someone else
                    if self.policy == LockPolicy::Require {
                        // Rollback any acquired locks
                        self.release_locks(&acquired);
                        return Err(WorkflowError::LockConflict {
                            resource: resource.clone(),
                            holder: result.holder,
                        });
                    }
                    // policy=Warn: log warning and continue
                    eprintln!(
                        "Warning: Could not acquire lock on {} (held by {:?})",
                        resource, result.holder
                    );
                }
                Err(e) => {
                    if self.policy == LockPolicy::Require {
                        // Rollback any acquired locks
                        self.release_locks(&acquired);
                        return Err(WorkflowError::LockFailed(format!(
                            "Failed to acquire lock on {}: {}",
                            resource, e
                        )));
                    }
                    // policy=Warn: log warning and continue
                    eprintln!(
                        "Warning: Lock acquisition failed for {}: {}",
                        resource, e
                    );
                }
            }
        }

        Ok(acquired)
    }

    /// Release previously acquired locks.
    ///
    /// This is a best-effort operation; individual failures are logged but don't
    /// cause the overall operation to fail.
    pub fn release_locks(&self, resources: &[String]) {
        for resource in resources {
            if let Err(e) = self.gritee.lock_release(resource) {
                eprintln!("Warning: Failed to release lock on {}: {}", resource, e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_policy_from_str() {
        assert_eq!(LockPolicy::from_str("off"), Some(LockPolicy::Off));
        assert_eq!(LockPolicy::from_str("warn"), Some(LockPolicy::Warn));
        assert_eq!(LockPolicy::from_str("require"), Some(LockPolicy::Require));
        assert_eq!(LockPolicy::from_str("invalid"), None);
    }

    #[test]
    fn test_lock_policy_default() {
        assert_eq!(LockPolicy::default(), LockPolicy::Warn);
    }
}
