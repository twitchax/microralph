//! PRD index generator.
//!
//! Scans the `.mr/prds/` directory for PRD files and generates an index
//! in `.mr/PRDS.md` with a table listing all PRDs.

use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::path::Path;
use std::sync::LazyLock;

use anyhow::{Context, Result};
use regex::Regex;

use super::{Prd, PrdStatus, UatStatus, parse_prd_file};

/// Pre-compiled regex pattern for extracting PRD references (e.g., "PRD-0001").
static PRD_REFERENCE_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"PRD-\d{4}").expect("PRD reference regex pattern is valid"));

/// Summary of a PRD for the index.
#[derive(Debug, Clone)]
pub struct PrdSummary {
    /// PRD ID (e.g., "PRD-0001").
    pub id: String,

    /// PRD title.
    pub title: String,

    /// PRD status.
    pub status: PrdStatus,

    /// Number of completed tasks.
    pub completed_tasks: usize,

    /// Total number of tasks.
    pub total_tasks: usize,

    /// Number of verified UATs.
    pub verified_uats: usize,

    /// Total number of UATs.
    pub total_uats: usize,

    /// Relative path to the PRD file from the index file's directory.
    pub relative_path: String,

    /// IDs of other PRDs referenced in this PRD's body.
    pub references: Vec<String>,

    /// PRD IDs this PRD depends on (from frontmatter `depends_on` field).
    pub depends_on: Vec<String>,
}

impl PrdSummary {
    /// Creates a new [`PrdSummary`] from a [`Prd`] and its file path.
    pub fn from_prd(prd: &Prd, relative_path: String) -> Self {
        let tasks = prd.tasks().unwrap_or_default();
        let completed_tasks = prd.completed_tasks().len();
        let total_tasks = tasks.len();

        // Count UATs.
        let acceptance_tests = prd
            .frontmatter
            .acceptance_tests
            .as_deref()
            .unwrap_or_default();
        let total_uats = acceptance_tests.len();
        let verified_uats = acceptance_tests
            .iter()
            .filter(|t| t.uat_status == UatStatus::Verified)
            .count();

        // Build searchable text from body and task notes.
        let mut searchable_text = prd.body.clone();

        for task in tasks {
            if let Some(notes) = &task.notes {
                searchable_text.push('\n');
                searchable_text.push_str(notes);
            }
        }

        let references = extract_prd_references(&searchable_text, prd.id());

        // Extract depends_on from frontmatter.
        let depends_on = prd.frontmatter.depends_on.clone().unwrap_or_default();

        Self {
            id: prd.id().to_string(),
            title: prd.title().to_string(),
            status: prd.status(),
            completed_tasks,
            total_tasks,
            verified_uats,
            total_uats,
            relative_path,
            references,
            depends_on,
        }
    }

    /// Returns a progress string like "3/10".
    pub fn progress(&self) -> String {
        format!("{}/{}", self.completed_tasks, self.total_tasks)
    }
}

/// Extracts references to other PRDs from a PRD's body text.
///
/// Searches for patterns like "PRD-0001", "PRD-0002", etc. and returns
/// a deduplicated, sorted list of referenced PRD IDs. Excludes self-references.
///
/// # Arguments
///
/// * `body` - The PRD body text to search
/// * `self_id` - The ID of the current PRD (to exclude self-references)
///
/// # Returns
///
/// A sorted vector of unique PRD IDs found in the body.
fn extract_prd_references(body: &str, self_id: &str) -> Vec<String> {
    let mut refs: HashSet<String> = HashSet::new();

    for cap in PRD_REFERENCE_PATTERN.find_iter(body) {
        let prd_id = cap.as_str().to_string();

        if prd_id != self_id {
            refs.insert(prd_id);
        }
    }

    let mut sorted: Vec<String> = refs.into_iter().collect();
    sorted.sort();
    sorted
}

/// Scans a directory for PRD files and returns parsed PRDs.
///
/// # Arguments
///
/// * `prds_dir` - Path to the directory containing PRD files (e.g., `.mr/prds/`)
///
/// # Returns
///
/// A vector of (filename, [`Prd`], `absolute_path`) tuples for successfully parsed PRDs.
/// Files that fail to parse are logged and skipped.
pub fn scan_prds(prds_dir: impl AsRef<Path>) -> Result<Vec<(String, Prd, std::path::PathBuf)>> {
    let prds_dir = prds_dir.as_ref();

    if !prds_dir.exists() {
        return Ok(Vec::new());
    }

    let mut prds = Vec::new();

    let entries = std::fs::read_dir(prds_dir)
        .with_context(|| format!("Failed to read PRDs directory: {}", prds_dir.display()))?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        // Only process .md files.
        if path.extension().is_some_and(|ext| ext == "md") {
            let filename = entry.file_name().to_string_lossy().into_owned();

            match parse_prd_file(&path) {
                Ok(prd) => {
                    prds.push((filename, prd, path));
                }
                Err(e) => {
                    tracing::warn!(
                        path = %path.display(),
                        error = %e,
                        "Failed to parse PRD file, skipping"
                    );
                }
            }
        }
    }

    // Sort by ID for deterministic output.
    prds.sort_by(|a, b| a.1.id().cmp(b.1.id()));

    Ok(prds)
}

/// Generates the PRDS.md index content.
///
/// # Arguments
///
/// * `prds` - Vector of (filename, [`Prd`], `absolute_path`) tuples
///
/// # Returns
///
/// The generated Markdown content for the index file.
pub fn generate_index(prds: &[(String, Prd, std::path::PathBuf)]) -> String {
    let mut output = String::new();

    // Header.
    output.push_str("# microralph — PRD Index\n\n");
    output.push_str("This file is auto-generated by `mr`. Do not edit manually.\n\n");

    // Group PRDs by status.
    let mut by_status: HashMap<PrdStatus, Vec<PrdSummary>> = HashMap::new();

    for (filename, prd, _abs_path) in prds {
        let relative_path = format!("prds/{filename}");
        let summary = PrdSummary::from_prd(prd, relative_path);
        by_status.entry(summary.status).or_default().push(summary);
    }

    // Active PRDs section.
    output.push_str("## Active PRDs\n\n");
    if let Some(active) = by_status.get(&PrdStatus::Active) {
        output.push_str(&generate_prd_table(active));
    } else {
        output.push_str("*No active PRDs.*\n");
    }
    output.push('\n');

    // Draft PRDs section.
    output.push_str("## Draft PRDs\n\n");
    if let Some(drafts) = by_status.get(&PrdStatus::Draft) {
        output.push_str(&generate_prd_table(drafts));
    } else {
        output.push_str("*No draft PRDs.*\n");
    }
    output.push('\n');

    // Done PRDs section.
    output.push_str("## Done PRDs\n\n");
    if let Some(done) = by_status.get(&PrdStatus::Done) {
        output.push_str(&generate_prd_table(done));
    } else {
        output.push_str("*No completed PRDs.*\n");
    }
    output.push('\n');

    // Parked PRDs section.
    output.push_str("## Parked PRDs\n\n");
    if let Some(parked) = by_status.get(&PrdStatus::Parked) {
        output.push_str(&generate_prd_table(parked));
    } else {
        output.push_str("*No parked PRDs.*\n");
    }
    output.push('\n');

    // Dependencies section (from frontmatter depends_on).
    let mut all_summaries: Vec<&PrdSummary> = by_status.values().flat_map(|v| v.iter()).collect();
    all_summaries.sort_by_key(|s| &s.id); // Sort by ID for deterministic ordering
    output.push_str(&generate_dependencies_section(&all_summaries));

    // Cross-References section.
    output.push_str(&generate_cross_references_section(&all_summaries));

    // Statistics.
    let total = prds.len();
    let active = by_status.get(&PrdStatus::Active).map_or(0, Vec::len);
    let draft = by_status.get(&PrdStatus::Draft).map_or(0, Vec::len);
    let done = by_status.get(&PrdStatus::Done).map_or(0, Vec::len);
    let parked = by_status.get(&PrdStatus::Parked).map_or(0, Vec::len);

    output.push_str("## Statistics\n\n");
    let _ = writeln!(output, "- **Total PRDs**: {total}");
    let _ = writeln!(output, "- **Active**: {active}");
    let _ = writeln!(output, "- **Draft**: {draft}");
    let _ = writeln!(output, "- **Done**: {done}");
    let _ = writeln!(output, "- **Parked**: {parked}");
    output.push('\n');

    // Footer.
    output.push_str("---\n\n");
    let _ = writeln!(
        output,
        "*Last updated: {}*",
        chrono::Local::now().format("%Y-%m-%d")
    );

    output
}

/// Generates a Markdown table for a list of PRD summaries.
fn generate_prd_table(summaries: &[PrdSummary]) -> String {
    let mut output = String::new();

    // Table header.
    output.push_str("| ID | Title | Status | Progress |\n");
    output.push_str("| -- | ----- | ------ | -------- |\n");

    // Table rows.
    for summary in summaries {
        let _ = writeln!(
            output,
            "| [{}]({}) | {} | {} | {} |",
            summary.id,
            summary.relative_path,
            summary.title,
            summary.status,
            summary.progress()
        );
    }

    output
}

/// Builds a lookup map from PRD ID to relative path.
fn build_path_map<'a>(summaries: &[&'a PrdSummary]) -> HashMap<&'a str, &'a str> {
    summaries
        .iter()
        .map(|s| (s.id.as_str(), s.relative_path.as_str()))
        .collect()
}

/// Formats PRD IDs as markdown links, falling back to plain text if not found.
fn format_prd_links(ids: &[String], path_map: &HashMap<&str, &str>) -> Vec<String> {
    ids.iter()
        .map(|id| {
            if let Some(path) = path_map.get(id.as_str()) {
                format!("[{id}]({path})")
            } else {
                id.clone()
            }
        })
        .collect()
}

/// Generates the Dependencies section showing PRD dependencies from frontmatter.
///
/// Lists which PRDs depend on other PRDs via the `depends_on` frontmatter field.
fn generate_dependencies_section(summaries: &[&PrdSummary]) -> String {
    let mut output = String::new();

    output.push_str("## Dependencies\n\n");

    let with_deps: Vec<_> = summaries
        .iter()
        .filter(|s| !s.depends_on.is_empty())
        .collect();

    if with_deps.is_empty() {
        output.push_str("*No PRD dependencies defined.*\n");
    } else {
        let path_map = build_path_map(summaries);

        for summary in with_deps {
            let deps_formatted = format_prd_links(&summary.depends_on, &path_map);

            let _ = writeln!(
                output,
                "- [{}]({}) depends on {}",
                summary.id,
                summary.relative_path,
                deps_formatted.join(", ")
            );
        }
    }
    output.push('\n');

    output
}

/// Generates the Cross-References section showing inter-PRD links.
///
/// Lists which PRDs reference other PRDs in their body text.
fn generate_cross_references_section(summaries: &[&PrdSummary]) -> String {
    let mut output = String::new();

    output.push_str("## Cross-References\n\n");

    let with_refs: Vec<_> = summaries
        .iter()
        .filter(|s| !s.references.is_empty())
        .collect();

    if with_refs.is_empty() {
        output.push_str("*No cross-references between PRDs.*\n");
    } else {
        let path_map = build_path_map(summaries);

        for summary in with_refs {
            let refs_formatted = format_prd_links(&summary.references, &path_map);

            let _ = writeln!(
                output,
                "- [{}]({}) → {}",
                summary.id,
                summary.relative_path,
                refs_formatted.join(", ")
            );
        }
    }
    output.push('\n');

    output
}

/// Generates the index file and writes it to disk.
///
/// # Arguments
///
/// * `prds_dir` - Path to the directory containing PRD files (e.g., `.mr/prds/`)
/// * `index_path` - Path where the index file should be written (e.g., `.mr/PRDS.md`)
///
/// # Returns
///
/// The number of PRDs included in the index.
pub fn generate_index_file(
    prds_dir: impl AsRef<Path>,
    index_path: impl AsRef<Path>,
) -> Result<usize> {
    let prds = scan_prds(prds_dir)?;
    let count = prds.len();
    let content = generate_index(&prds);

    std::fs::write(index_path.as_ref(), content).with_context(|| {
        format!(
            "Failed to write index file: {}",
            index_path.as_ref().display()
        )
    })?;

    Ok(count)
}

/// Scans PRDs from a repository root and returns [`PrdSummary`] objects.
///
/// # Arguments
///
/// * `root` - The repository root directory
///
/// # Returns
///
/// A vector of [`PrdSummary`] objects for all successfully parsed PRDs.
pub fn scan_prd_summaries(root: impl AsRef<Path>) -> Result<Vec<PrdSummary>> {
    let prds_dir = root.as_ref().join(".mr").join("prds");
    let prds = scan_prds(&prds_dir)?;

    Ok(prds
        .into_iter()
        .map(|(filename, prd, _abs_path)| {
            let relative_path = format!("prds/{filename}");
            PrdSummary::from_prd(&prd, relative_path)
        })
        .collect())
}

/// Generates the index file from a repository root.
///
/// Convenience function that determines paths from the root.
///
/// # Arguments
///
/// * `root` - The repository root directory
///
/// # Returns
///
/// The number of PRDs included in the index.
pub fn generate_index_from_root(root: impl AsRef<Path>) -> Result<usize> {
    let root = root.as_ref();
    let prds_dir = root.join(".mr").join("prds");
    let index_path = root.join(".mr").join("PRDS.md");

    generate_index_file(prds_dir, index_path)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::prd::types::{PrdFrontmatter, Task, TaskStatus};

    fn make_test_prd(id: &str, title: &str, status: PrdStatus, tasks: Vec<Task>) -> Prd {
        let frontmatter = PrdFrontmatter {
            id: id.to_string(),
            title: title.to_string(),
            status,
            tasks: if tasks.is_empty() { None } else { Some(tasks) },
            ..Default::default()
        };

        Prd::new(frontmatter, "# Body\n".to_string())
    }

    fn make_task(id: &str, priority: u32, status: TaskStatus) -> Task {
        Task {
            id: id.to_string(),
            title: format!("Task {}", id),
            priority,
            status,
            notes: None,
        }
    }

    #[test]
    fn test_prd_summary_progress() {
        let prd = make_test_prd(
            "PRD-0001",
            "Test",
            PrdStatus::Active,
            vec![
                make_task("T-001", 1, TaskStatus::Done),
                make_task("T-002", 2, TaskStatus::Done),
                make_task("T-003", 3, TaskStatus::Todo),
            ],
        );

        let summary = PrdSummary::from_prd(&prd, "prds/test.md".to_string());

        assert_eq!(summary.completed_tasks, 2);
        assert_eq!(summary.total_tasks, 3);
        assert_eq!(summary.progress(), "2/3");
    }

    #[test]
    fn test_prd_summary_no_tasks() {
        let prd = make_test_prd("PRD-0001", "Test", PrdStatus::Draft, vec![]);

        let summary = PrdSummary::from_prd(&prd, "prds/test.md".to_string());

        assert_eq!(summary.completed_tasks, 0);
        assert_eq!(summary.total_tasks, 0);
        assert_eq!(summary.progress(), "0/0");
    }

    #[test]
    fn test_generate_prd_table() {
        let summaries = vec![
            PrdSummary {
                id: "PRD-0001".to_string(),
                title: "First PRD".to_string(),
                status: PrdStatus::Active,
                completed_tasks: 2,
                total_tasks: 5,
                verified_uats: 0,
                total_uats: 0,
                relative_path: "prds/PRD-0001.md".to_string(),
                references: vec![],
                depends_on: vec![],
            },
            PrdSummary {
                id: "PRD-0002".to_string(),
                title: "Second PRD".to_string(),
                status: PrdStatus::Active,
                completed_tasks: 0,
                total_tasks: 3,
                verified_uats: 0,
                total_uats: 0,
                relative_path: "prds/PRD-0002.md".to_string(),
                references: vec![],
                depends_on: vec![],
            },
        ];

        let table = generate_prd_table(&summaries);

        assert!(table.contains("[PRD-0001](prds/PRD-0001.md)"));
        assert!(table.contains("First PRD"));
        assert!(table.contains("2/5"));
        assert!(table.contains("[PRD-0002](prds/PRD-0002.md)"));
        assert!(table.contains("Second PRD"));
        assert!(table.contains("0/3"));
    }

    #[test]
    fn test_generate_index_empty() {
        let prds: Vec<(String, Prd, std::path::PathBuf)> = vec![];
        let index = generate_index(&prds);

        assert!(index.contains("# microralph — PRD Index"));
        assert!(index.contains("*No active PRDs.*"));
        assert!(index.contains("*No draft PRDs.*"));
        assert!(index.contains("*No completed PRDs.*"));
        assert!(index.contains("*No parked PRDs.*"));
        assert!(index.contains("*No PRD dependencies defined.*"));
        assert!(index.contains("**Total PRDs**: 0"));
    }

    #[test]
    fn test_generate_index_with_prds() {
        let prds = vec![
            (
                "PRD-0001.md".to_string(),
                make_test_prd(
                    "PRD-0001",
                    "Active PRD",
                    PrdStatus::Active,
                    vec![
                        make_task("T-001", 1, TaskStatus::Done),
                        make_task("T-002", 2, TaskStatus::Todo),
                    ],
                ),
                std::path::PathBuf::from("prds/PRD-0001.md"),
            ),
            (
                "PRD-0002.md".to_string(),
                make_test_prd("PRD-0002", "Draft PRD", PrdStatus::Draft, vec![]),
                std::path::PathBuf::from("prds/PRD-0002.md"),
            ),
            (
                "PRD-0003.md".to_string(),
                make_test_prd(
                    "PRD-0003",
                    "Done PRD",
                    PrdStatus::Done,
                    vec![make_task("T-001", 1, TaskStatus::Done)],
                ),
                std::path::PathBuf::from("prds/PRD-0003.md"),
            ),
        ];

        let index = generate_index(&prds);

        // Check sections.
        assert!(index.contains("## Active PRDs"));
        assert!(index.contains("[PRD-0001](prds/PRD-0001.md)"));
        assert!(index.contains("Active PRD"));
        assert!(index.contains("1/2"));

        assert!(index.contains("## Draft PRDs"));
        assert!(index.contains("[PRD-0002](prds/PRD-0002.md)"));
        assert!(index.contains("Draft PRD"));

        assert!(index.contains("## Done PRDs"));
        assert!(index.contains("[PRD-0003](prds/PRD-0003.md)"));
        assert!(index.contains("Done PRD"));

        // Check statistics.
        assert!(index.contains("**Total PRDs**: 3"));
        assert!(index.contains("**Active**: 1"));
        assert!(index.contains("**Draft**: 1"));
        assert!(index.contains("**Done**: 1"));
        assert!(index.contains("**Parked**: 0"));
    }

    #[test]
    fn test_scan_prds_from_actual_directory() {
        // Use the actual .mr/prds directory from this repo.
        let prds_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".mr/prds");

        if prds_dir.exists() {
            let prds = scan_prds(&prds_dir).unwrap();

            // Should find at least PRD-0001.
            assert!(!prds.is_empty());

            let prd_0001 = prds.iter().find(|(_, p, _)| p.id() == "PRD-0001");

            assert!(prd_0001.is_some());
        }
    }

    #[test]
    fn test_scan_prds_nonexistent_directory() {
        let prds = scan_prds("/nonexistent/path/to/prds").unwrap();

        assert!(prds.is_empty());
    }

    #[test]
    fn test_extract_prd_references_none() {
        let body = "# Summary\n\nNo references here.";
        let refs = extract_prd_references(body, "PRD-0001");

        assert!(refs.is_empty());
    }

    #[test]
    fn test_extract_prd_references_single() {
        let body = "# Summary\n\nThis depends on PRD-0002.";
        let refs = extract_prd_references(body, "PRD-0001");

        assert_eq!(refs, vec!["PRD-0002"]);
    }

    #[test]
    fn test_extract_prd_references_multiple() {
        let body = "# Summary\n\nSee PRD-0002 and PRD-0003 for context. Also PRD-0005.";
        let refs = extract_prd_references(body, "PRD-0001");

        assert_eq!(refs, vec!["PRD-0002", "PRD-0003", "PRD-0005"]);
    }

    #[test]
    fn test_extract_prd_references_excludes_self() {
        let body = "# Summary\n\nPRD-0001 references PRD-0002 and itself.";
        let refs = extract_prd_references(body, "PRD-0001");

        assert_eq!(refs, vec!["PRD-0002"]);
    }

    #[test]
    fn test_extract_prd_references_deduplicates() {
        let body = "# Summary\n\nPRD-0002 appears twice. See PRD-0002 again.";
        let refs = extract_prd_references(body, "PRD-0001");

        assert_eq!(refs, vec!["PRD-0002"]);
    }

    #[test]
    fn test_generate_cross_references_no_refs() {
        let summary = PrdSummary {
            id: "PRD-0001".to_string(),
            title: "Test PRD".to_string(),
            status: PrdStatus::Active,
            completed_tasks: 0,
            total_tasks: 0,
            verified_uats: 0,
            total_uats: 0,
            relative_path: "prds/PRD-0001.md".to_string(),
            references: vec![],
            depends_on: vec![],
        };

        let section = generate_cross_references_section(&[&summary]);

        assert!(section.contains("## Cross-References"));
        assert!(section.contains("*No cross-references between PRDs.*"));
    }

    #[test]
    fn test_generate_cross_references_with_refs() {
        let summary1 = PrdSummary {
            id: "PRD-0001".to_string(),
            title: "First PRD".to_string(),
            status: PrdStatus::Active,
            completed_tasks: 0,
            total_tasks: 0,
            verified_uats: 0,
            total_uats: 0,
            relative_path: "prds/PRD-0001.md".to_string(),
            references: vec!["PRD-0002".to_string()],
            depends_on: vec![],
        };
        let summary2 = PrdSummary {
            id: "PRD-0002".to_string(),
            title: "Second PRD".to_string(),
            status: PrdStatus::Done,
            completed_tasks: 1,
            total_tasks: 1,
            verified_uats: 0,
            total_uats: 0,
            relative_path: "prds/PRD-0002.md".to_string(),
            references: vec![],
            depends_on: vec![],
        };

        let section = generate_cross_references_section(&[&summary1, &summary2]);

        assert!(section.contains("## Cross-References"));
        assert!(section.contains("[PRD-0001](prds/PRD-0001.md) → [PRD-0002](prds/PRD-0002.md)"));
        assert!(!section.contains("*No cross-references"));
    }

    #[test]
    fn test_prd_summary_extracts_references() {
        let frontmatter = PrdFrontmatter {
            id: "PRD-0001".to_string(),
            title: "Test PRD".to_string(),
            ..Default::default()
        };
        let prd = Prd::new(frontmatter, "This depends on PRD-0002.".to_string());
        let summary = PrdSummary::from_prd(&prd, "prds/PRD-0001.md".to_string());

        assert_eq!(summary.references, vec!["PRD-0002"]);
    }

    #[test]
    fn test_prd_summary_extracts_depends_on() {
        let frontmatter = PrdFrontmatter {
            id: "PRD-0003".to_string(),
            title: "Test PRD".to_string(),
            depends_on: Some(vec!["PRD-0001".to_string(), "PRD-0002".to_string()]),
            ..Default::default()
        };
        let prd = Prd::new(frontmatter, "No body references.".to_string());
        let summary = PrdSummary::from_prd(&prd, "prds/PRD-0003.md".to_string());

        assert_eq!(
            summary.depends_on,
            vec!["PRD-0001".to_string(), "PRD-0002".to_string()]
        );
    }

    #[test]
    fn test_prd_summary_depends_on_empty() {
        let frontmatter = PrdFrontmatter {
            id: "PRD-0001".to_string(),
            title: "Test PRD".to_string(),
            ..Default::default()
        };
        let prd = Prd::new(frontmatter, "Body text.".to_string());
        let summary = PrdSummary::from_prd(&prd, "prds/PRD-0001.md".to_string());

        assert!(summary.depends_on.is_empty());
    }

    #[test]
    fn test_generate_dependencies_no_deps() {
        let summary = PrdSummary {
            id: "PRD-0001".to_string(),
            title: "Test PRD".to_string(),
            status: PrdStatus::Active,
            completed_tasks: 0,
            total_tasks: 0,
            verified_uats: 0,
            total_uats: 0,
            relative_path: "prds/PRD-0001.md".to_string(),
            references: vec![],
            depends_on: vec![],
        };

        let section = generate_dependencies_section(&[&summary]);

        assert!(section.contains("## Dependencies"));
        assert!(section.contains("*No PRD dependencies defined.*"));
    }

    #[test]
    fn test_generate_dependencies_with_deps() {
        let summary1 = PrdSummary {
            id: "PRD-0001".to_string(),
            title: "First PRD".to_string(),
            status: PrdStatus::Done,
            completed_tasks: 1,
            total_tasks: 1,
            verified_uats: 0,
            total_uats: 0,
            relative_path: "prds/PRD-0001.md".to_string(),
            references: vec![],
            depends_on: vec![],
        };
        let summary2 = PrdSummary {
            id: "PRD-0002".to_string(),
            title: "Second PRD".to_string(),
            status: PrdStatus::Active,
            completed_tasks: 0,
            total_tasks: 2,
            verified_uats: 0,
            total_uats: 0,
            relative_path: "prds/PRD-0002.md".to_string(),
            references: vec![],
            depends_on: vec!["PRD-0001".to_string()],
        };

        let section = generate_dependencies_section(&[&summary1, &summary2]);

        assert!(section.contains("## Dependencies"));
        assert!(
            section
                .contains("[PRD-0002](prds/PRD-0002.md) depends on [PRD-0001](prds/PRD-0001.md)")
        );
        assert!(!section.contains("*No PRD dependencies"));
    }

    #[test]
    fn test_generate_dependencies_multiple_deps() {
        let summary1 = PrdSummary {
            id: "PRD-0001".to_string(),
            title: "First PRD".to_string(),
            status: PrdStatus::Done,
            completed_tasks: 1,
            total_tasks: 1,
            verified_uats: 0,
            total_uats: 0,
            relative_path: "prds/PRD-0001.md".to_string(),
            references: vec![],
            depends_on: vec![],
        };
        let summary2 = PrdSummary {
            id: "PRD-0002".to_string(),
            title: "Second PRD".to_string(),
            status: PrdStatus::Done,
            completed_tasks: 1,
            total_tasks: 1,
            verified_uats: 0,
            total_uats: 0,
            relative_path: "prds/PRD-0002.md".to_string(),
            references: vec![],
            depends_on: vec![],
        };
        let summary3 = PrdSummary {
            id: "PRD-0003".to_string(),
            title: "Third PRD".to_string(),
            status: PrdStatus::Active,
            completed_tasks: 0,
            total_tasks: 3,
            verified_uats: 0,
            total_uats: 0,
            relative_path: "prds/PRD-0003.md".to_string(),
            references: vec![],
            depends_on: vec!["PRD-0001".to_string(), "PRD-0002".to_string()],
        };

        let section = generate_dependencies_section(&[&summary1, &summary2, &summary3]);

        assert!(section.contains("## Dependencies"));
        assert!(section.contains("[PRD-0003](prds/PRD-0003.md) depends on [PRD-0001](prds/PRD-0001.md), [PRD-0002](prds/PRD-0002.md)"));
    }
}
