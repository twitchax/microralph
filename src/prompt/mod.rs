//! Prompt library and placeholder expansion system.
//!
//! This module provides:
//! - Loading of static prompt files from `.mr/prompts/`
//! - Placeholder expansion using `{{variable}}` syntax
//! - Type-safe prompt kinds for all Micro Ralph stages

mod expand;
mod loader;
mod types;

pub use expand::{PlaceholderContext, PlaceholderValue, expand_placeholders};
pub use loader::load_prompt_with_fallback;
pub use types::PromptKind;
