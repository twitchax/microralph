//! AI-driven PRD suggestion generation.
//!
//! This module implements `mr suggest` which analyzes the codebase,
//! existing PRDs, and external research to generate actionable PRD suggestions.

use anyhow::Result;
use std::path::Path;

use crate::runner::Runner;

/// Runs the PRD suggestion flow.
///
/// This function will be implemented in task T-004 to:
/// 1. Analyze the codebase and existing PRDs
/// 2. Invoke the runner to generate 5 PRD suggestions
/// 3. Display a numbered picker for user selection
/// 4. Flow the selected suggestion into `mr new` with pre-filled context
pub fn suggest<R>(_root: &Path, _runner: &R) -> Result<()>
where
    R: Runner + ?Sized,
{
    // Placeholder implementation - to be completed in T-004.
    println!("Suggest command placeholder - implementation coming in T-004");
    Ok(())
}
