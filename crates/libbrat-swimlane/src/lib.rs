//! Brat Swimlane Library
//!
//! Provides swimlane scheduling for parallel agent teams during the
//! Implementation phase of the software factory pipeline.
//!
//! A swimlane is a vertical lane in a kanban board representing a
//! parallel agent team with its own worktree, engine, and task queue.

pub mod error;
pub mod lane;
pub mod scheduler;

pub use error::SwimlaneError;
pub use lane::{Swimlane, TaskAssignment};
pub use scheduler::{ScheduleResult, SwimlaneScheduler};
