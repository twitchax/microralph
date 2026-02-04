//! PRD (Product Requirements Document) file format and parser.
//!
//! PRDs are Markdown files with YAML frontmatter containing metadata about
//! tasks, status, and other structured information. The parser preserves
//! human-written Markdown content during round-trips.

pub mod edit;
pub mod finalize;
mod index;
pub mod new;
mod parser;
pub mod types;

pub use index::{PrdSummary, generate_index_from_root, scan_prd_summaries, scan_prds};
pub use parser::{parse_prd, parse_prd_file, serialize_prd};
pub use types::{AcceptanceTest, Prd, PrdStatus, TaskStatus, UatStatus};

// Re-exports used only in tests.
#[cfg(test)]
pub use types::{PrdFrontmatter, Task};
