//! PRD (Product Requirements Document) file format and parser.
//!
//! PRDs are Markdown files with YAML frontmatter containing metadata about
//! tasks, status, and other structured information. The parser preserves
//! human-written Markdown content during round-trips.

mod index;
mod parser;
mod types;

pub use index::{PrdSummary, generate_index_from_root, scan_prd_summaries};
pub use parser::{parse_prd, parse_prd_file};
pub use types::{Prd, PrdStatus};
