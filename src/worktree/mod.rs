//! Worktree orchestration module.
//!
//! Provides types, state management, git helpers, IPC, and daemon logic
//! for parallel PRD execution via git worktrees.

pub mod daemon;
pub mod git;
pub mod ipc;
pub mod state;
pub mod types;
