//! Shared utilities for microralph.
//!
//! This module contains common utilities used across the codebase:
//! - `changelog`: Changelog management for Keep a Changelog format
//! - `colors`: Terminal output styling
//! - `spinner`: Progress spinners for long-running operations
//! - `qa_workflow`: Question/answer workflow utilities for PRD operations

pub mod changelog;
pub mod colors;
pub mod qa_workflow;
pub mod spinner;
