//! HTTP API for bratd daemon.
//!
//! Provides REST endpoints for managing brat across multiple repositories.
//! A single `bratd` daemon can serve multiple concurrent brat CLI sessions
//! and a Vue.js dashboard.

pub mod routes;
pub mod server;
pub mod state;
pub mod watcher;

pub use server::run_server;
pub use state::DaemonState;
