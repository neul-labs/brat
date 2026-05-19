//! Gritee integration library for Brat.
//!
//! This crate provides a client for interacting with the Gritee CLI
//! to manage convoys and tasks.
//!
//! # Example
//!
//! ```ignore
//! use libbrat_grite::{GriteeClient, TaskStatus};
//!
//! let client = GriteeClient::new("/path/to/repo");
//!
//! // Create a convoy
//! let convoy = client.convoy_create("Feature: Dark mode", None)?;
//!
//! // Create a task in the convoy
//! let task = client.task_create(&convoy.convoy_id, "Implement toggle", None)?;
//!
//! // Update task status
//! client.task_update_status(&task.task_id, TaskStatus::Running)?;
//! ```

mod client;
mod error;
mod id;
pub mod reconcile;
pub mod state_machine;
mod types;

pub use client::{GriteeClient, LockResult};
pub use error::GriteeError;
pub use id::{
    generate_convoy_id, generate_session_id, generate_task_id, is_valid_convoy_id,
    is_valid_session_id, is_valid_task_id, parse_convoy_id, parse_session_id, parse_task_id,
};
pub use state_machine::{State, StateMachine, Transition, TransitionError};
pub use types::{
    ContextIndexResult, Convoy, ConvoyStatus, DependencyType, FileContext, GriteeIssue,
    GriteeIssueSummary, ProjectContextEntry, Session, SessionRole, SessionStatus, SessionType,
    Symbol, SymbolMatch, Task, TaskDependency, TaskStatus,
};
