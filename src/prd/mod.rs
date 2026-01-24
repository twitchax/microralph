//! PRD (Product Requirements Document) file format and parser.
//!
//! PRDs are Markdown files with YAML frontmatter containing metadata about
//! tasks, status, and other structured information. The parser preserves
//! human-written Markdown content during round-trips.

// Allow warnings since this module is newly added and not yet fully integrated.
#![allow(dead_code, unused_imports)]

mod index;
mod parser;
mod types;

pub use index::{PrdSummary, generate_index, generate_index_file, scan_prds};
pub use parser::{parse_prd, parse_prd_file, serialize_prd};
pub use types::{Prd, PrdFrontmatter, PrdStatus, Task, TaskStatus};
