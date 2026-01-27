//! Graph module for PRD dependency visualization.
//!
//! Provides data structures and functions for building a dependency graph
//! from PRD `depends_on` fields. The graph is used by rendering commands
//! (ascii, mermaid, dot) to visualize PRD relationships.

// Allow dead code: public APIs will be used by graph rendering commands in T-009, T-010, T-011, T-012.
#![allow(dead_code)]

use std::collections::HashSet;
use std::path::Path;

use anyhow::Result;

use crate::prd::{PrdStatus, PrdSummary, scan_prds};

/// A node in the PRD dependency graph.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphNode {
    /// PRD ID (e.g., "PRD-0001").
    pub id: String,

    /// PRD title.
    pub title: String,

    /// PRD status.
    pub status: PrdStatus,

    /// Whether this node represents a missing PRD (referenced but not found).
    pub is_missing: bool,
}

impl GraphNode {
    /// Creates a node from a PrdSummary.
    pub fn from_summary(summary: &PrdSummary) -> Self {
        Self {
            id: summary.id.clone(),
            title: summary.title.clone(),
            status: summary.status,
            is_missing: false,
        }
    }

    /// Creates a placeholder node for a missing PRD reference.
    pub fn missing(id: &str) -> Self {
        Self {
            id: id.to_string(),
            title: format!("{} (not found)", id),
            status: PrdStatus::default(),
            is_missing: true,
        }
    }
}

/// An edge in the PRD dependency graph.
///
/// Represents a directed edge from `from` to `to`, meaning
/// "the PRD with id `from` depends on the PRD with id `to`".
#[derive(Debug, Clone, PartialEq)]
pub struct GraphEdge {
    /// The dependent PRD ID (the one that has `depends_on`).
    pub from: String,

    /// The dependency PRD ID (the one being depended upon).
    pub to: String,

    /// Whether this edge points to a missing PRD.
    pub is_missing: bool,
}

/// A complete PRD dependency graph.
#[derive(Debug, Clone)]
pub struct PrdGraph {
    /// All nodes in the graph (both existing and missing PRDs).
    pub nodes: Vec<GraphNode>,

    /// All edges in the graph.
    pub edges: Vec<GraphEdge>,

    /// PRD IDs that were referenced but not found.
    pub missing_refs: Vec<String>,

    /// Warnings generated during graph construction.
    pub warnings: Vec<String>,
}

impl PrdGraph {
    /// Returns true if the graph has any missing references.
    pub fn has_missing_refs(&self) -> bool {
        !self.missing_refs.is_empty()
    }

    /// Returns the number of nodes (excluding missing refs).
    pub fn node_count(&self) -> usize {
        self.nodes.iter().filter(|n| !n.is_missing).count()
    }

    /// Returns the number of edges.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
}

/// Builds a dependency graph from PRDs in the repository.
///
/// Scans all PRDs in `.mr/prds/`, extracts their `depends_on` fields,
/// and constructs a graph. Missing references (PRD IDs in `depends_on`
/// that don't exist) are logged as warnings and represented as
/// placeholder nodes with `is_missing: true`.
///
/// # Arguments
///
/// * `root` - The repository root directory.
///
/// # Returns
///
/// A `PrdGraph` containing nodes, edges, and any warnings about missing refs.
pub fn build_graph(root: impl AsRef<Path>) -> Result<PrdGraph> {
    let prds_dir = root.as_ref().join(".mr").join("prds");
    let prds = scan_prds(&prds_dir)?;

    build_graph_from_prds(&prds)
}

/// Builds a dependency graph from parsed PRDs.
///
/// This is the main implementation that works with full Prd objects
/// which have access to the `depends_on` field.
///
/// # Arguments
///
/// * `prds` - Slice of (filename, Prd, path) tuples from `scan_prds`.
///
/// # Returns
///
/// A `PrdGraph` containing nodes, edges, and any warnings about missing refs.
pub fn build_graph_from_prds(
    prds: &[(String, crate::prd::Prd, std::path::PathBuf)],
) -> Result<PrdGraph> {
    // Build a lookup map of existing PRD IDs.
    let existing_ids: HashSet<&str> = prds.iter().map(|(_, prd, _)| prd.id()).collect();

    // Track missing references.
    let mut missing_refs: HashSet<String> = HashSet::new();
    let mut warnings: Vec<String> = Vec::new();

    // Build nodes from PRDs.
    let mut nodes: Vec<GraphNode> = prds
        .iter()
        .map(|(_, prd, _)| GraphNode {
            id: prd.id().to_string(),
            title: prd.title().to_string(),
            status: prd.status(),
            is_missing: false,
        })
        .collect();

    // Build edges from depends_on fields.
    let mut edges: Vec<GraphEdge> = Vec::new();

    for (_, prd, _) in prds {
        if let Some(depends_on) = &prd.frontmatter.depends_on {
            for dep_id in depends_on {
                let is_missing = !existing_ids.contains(dep_id.as_str());

                if is_missing {
                    let warning =
                        format!("{} depends on {} which does not exist", prd.id(), dep_id);
                    tracing::warn!("{}", warning);
                    warnings.push(warning);
                    missing_refs.insert(dep_id.clone());
                }

                edges.push(GraphEdge {
                    from: prd.id().to_string(),
                    to: dep_id.clone(),
                    is_missing,
                });
            }
        }
    }

    // Add placeholder nodes for missing references.
    let mut missing_refs_sorted: Vec<String> = missing_refs.iter().cloned().collect();
    missing_refs_sorted.sort();

    for missing_id in &missing_refs_sorted {
        nodes.push(GraphNode::missing(missing_id));
    }

    // Sort nodes by ID for deterministic output.
    nodes.sort_by(|a, b| a.id.cmp(&b.id));

    // Sort edges for deterministic output.
    edges.sort_by(|a, b| (&a.from, &a.to).cmp(&(&b.from, &b.to)));

    Ok(PrdGraph {
        nodes,
        edges,
        missing_refs: missing_refs_sorted,
        warnings,
    })
}

// ============================================================================
// ASCII Rendering
// ============================================================================

use std::collections::HashMap;

/// Configuration for ASCII graph rendering.
#[derive(Debug, Clone)]
pub struct AsciiConfig {
    /// Whether to show node titles in addition to IDs.
    pub show_titles: bool,

    /// Maximum title length before truncation.
    pub max_title_len: usize,
}

impl Default for AsciiConfig {
    fn default() -> Self {
        Self {
            show_titles: true,
            max_title_len: 40,
        }
    }
}

impl AsciiConfig {
    /// Creates a new config with defaults (show titles, max 40 chars).
    pub fn new() -> Self {
        Self::default()
    }
}

/// Renders the graph as ASCII art.
///
/// The output shows each node with its dependencies listed below it.
/// Missing nodes are shown with dashed borders: `- - -`.
/// Normal nodes use solid borders: `[   ]`.
///
/// Example output:
/// ```text
/// PRD Dependency Graph
/// ====================
///
/// [PRD-0001] First PRD (done)
///
/// [PRD-0002] Second PRD (active)
///   └── PRD-0001
///
/// - PRD-9999 - (not found)
///   ⚠ Referenced by: PRD-0002
/// ```
///
/// # Arguments
///
/// * `graph` - The dependency graph to render.
/// * `config` - Configuration for rendering (optional).
///
/// # Returns
///
/// A string containing the ASCII representation of the graph.
pub fn render_ascii(graph: &PrdGraph, config: Option<AsciiConfig>) -> String {
    let config = config.unwrap_or_default();
    let mut output = String::new();

    // Header.
    output.push_str("PRD Dependency Graph\n");
    output.push_str("====================\n\n");

    // Quick check for empty graph.
    if graph.nodes.is_empty() {
        output.push_str("(no PRDs found)\n");
        return output;
    }

    // Build a map of dependencies for each node (what it depends on).
    let deps_map: HashMap<&str, Vec<&str>> =
        graph.edges.iter().fold(HashMap::new(), |mut acc, edge| {
            acc.entry(edge.from.as_str())
                .or_default()
                .push(edge.to.as_str());
            acc
        });

    // Build a map of reverse dependencies (what depends on this node).
    let reverse_deps_map: HashMap<&str, Vec<&str>> =
        graph.edges.iter().fold(HashMap::new(), |mut acc, edge| {
            acc.entry(edge.to.as_str())
                .or_default()
                .push(edge.from.as_str());
            acc
        });

    // First, render non-missing nodes.
    for node in graph.nodes.iter().filter(|n| !n.is_missing) {
        render_node(&mut output, node, &deps_map, &config);
    }

    // Then, render missing nodes separately with warnings.
    let missing_nodes: Vec<_> = graph.nodes.iter().filter(|n| n.is_missing).collect();
    if !missing_nodes.is_empty() {
        output.push_str("--- Missing References ---\n\n");

        for node in missing_nodes {
            render_missing_node(&mut output, node, &reverse_deps_map);
        }
    }

    // Summary stats at the end.
    output.push_str(&format!(
        "---\n{} PRDs, {} dependencies",
        graph.node_count(),
        graph.edge_count()
    ));

    if graph.has_missing_refs() {
        output.push_str(&format!(", {} missing", graph.missing_refs.len()));
    }

    output.push('\n');

    output
}

/// Renders a single node with its dependencies.
fn render_node(
    output: &mut String,
    node: &GraphNode,
    deps_map: &HashMap<&str, Vec<&str>>,
    config: &AsciiConfig,
) {
    // Node header: [PRD-XXXX] Title (status)
    let title_display = if config.show_titles {
        let title = if node.title.len() > config.max_title_len {
            format!("{}...", &node.title[..config.max_title_len - 3])
        } else {
            node.title.clone()
        };
        format!(" {}", title)
    } else {
        String::new()
    };

    output.push_str(&format!(
        "[{}]{} ({})\n",
        node.id, title_display, node.status
    ));

    // List dependencies.
    if let Some(deps) = deps_map.get(node.id.as_str()) {
        let dep_count = deps.len();
        for (i, dep) in deps.iter().enumerate() {
            let connector = if i == dep_count - 1 {
                "└──"
            } else {
                "├──"
            };
            output.push_str(&format!("  {} {}\n", connector, dep));
        }
    }

    output.push('\n');
}

/// Renders a missing node with warning about what references it.
fn render_missing_node(
    output: &mut String,
    node: &GraphNode,
    reverse_deps_map: &HashMap<&str, Vec<&str>>,
) {
    // Missing node: - PRD-XXXX - (not found)
    output.push_str(&format!("- {} - (not found)\n", node.id));

    // Show what references this missing node.
    if let Some(refs) = reverse_deps_map.get(node.id.as_str()) {
        output.push_str(&format!("  ⚠ Referenced by: {}\n", refs.join(", ")));
    }

    output.push('\n');
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::prd::Prd;
    use crate::prd::types::PrdFrontmatter;

    fn make_test_prd(
        id: &str,
        title: &str,
        status: PrdStatus,
        depends_on: Option<Vec<String>>,
    ) -> Prd {
        let frontmatter = PrdFrontmatter {
            id: id.to_string(),
            title: title.to_string(),
            status,
            depends_on,
            ..Default::default()
        };

        Prd::new(frontmatter, "# Body\n".to_string())
    }

    #[test]
    fn test_graph_node_from_summary() {
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
        };

        let node = GraphNode::from_summary(&summary);

        assert_eq!(node.id, "PRD-0001");
        assert_eq!(node.title, "Test PRD");
        assert_eq!(node.status, PrdStatus::Active);
        assert!(!node.is_missing);
    }

    #[test]
    fn test_graph_node_missing() {
        let node = GraphNode::missing("PRD-9999");

        assert_eq!(node.id, "PRD-9999");
        assert_eq!(node.title, "PRD-9999 (not found)");
        assert!(node.is_missing);
    }

    #[test]
    fn test_build_graph_no_dependencies() {
        let prds = vec![
            (
                "PRD-0001.md".to_string(),
                make_test_prd("PRD-0001", "First PRD", PrdStatus::Active, None),
                std::path::PathBuf::from("prds/PRD-0001.md"),
            ),
            (
                "PRD-0002.md".to_string(),
                make_test_prd("PRD-0002", "Second PRD", PrdStatus::Draft, None),
                std::path::PathBuf::from("prds/PRD-0002.md"),
            ),
        ];

        let graph = build_graph_from_prds(&prds).unwrap();

        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 0);
        assert!(graph.missing_refs.is_empty());
        assert!(graph.warnings.is_empty());
        assert!(!graph.has_missing_refs());
    }

    #[test]
    fn test_build_graph_with_valid_dependencies() {
        let prds = vec![
            (
                "PRD-0001.md".to_string(),
                make_test_prd("PRD-0001", "First PRD", PrdStatus::Done, None),
                std::path::PathBuf::from("prds/PRD-0001.md"),
            ),
            (
                "PRD-0002.md".to_string(),
                make_test_prd(
                    "PRD-0002",
                    "Second PRD",
                    PrdStatus::Active,
                    Some(vec!["PRD-0001".to_string()]),
                ),
                std::path::PathBuf::from("prds/PRD-0002.md"),
            ),
        ];

        let graph = build_graph_from_prds(&prds).unwrap();

        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);

        let edge = &graph.edges[0];
        assert_eq!(edge.from, "PRD-0002");
        assert_eq!(edge.to, "PRD-0001");
        assert!(!edge.is_missing);

        assert!(graph.missing_refs.is_empty());
        assert!(!graph.has_missing_refs());
    }

    #[test]
    fn test_build_graph_with_missing_dependency() {
        let prds = vec![(
            "PRD-0001.md".to_string(),
            make_test_prd(
                "PRD-0001",
                "First PRD",
                PrdStatus::Active,
                Some(vec!["PRD-9999".to_string()]),
            ),
            std::path::PathBuf::from("prds/PRD-0001.md"),
        )];

        let graph = build_graph_from_prds(&prds).unwrap();

        // Should have 2 nodes: PRD-0001 and the missing PRD-9999.
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);

        // Check the missing node.
        let missing_node = graph.nodes.iter().find(|n| n.id == "PRD-9999").unwrap();
        assert!(missing_node.is_missing);
        assert!(missing_node.title.contains("not found"));

        // Check the edge.
        let edge = &graph.edges[0];
        assert_eq!(edge.from, "PRD-0001");
        assert_eq!(edge.to, "PRD-9999");
        assert!(edge.is_missing);

        // Check warnings.
        assert!(graph.has_missing_refs());
        assert_eq!(graph.missing_refs, vec!["PRD-9999"]);
        assert_eq!(graph.warnings.len(), 1);
        assert!(graph.warnings[0].contains("does not exist"));
    }

    #[test]
    fn test_build_graph_multiple_dependencies() {
        let prds = vec![
            (
                "PRD-0001.md".to_string(),
                make_test_prd("PRD-0001", "First PRD", PrdStatus::Done, None),
                std::path::PathBuf::from("prds/PRD-0001.md"),
            ),
            (
                "PRD-0002.md".to_string(),
                make_test_prd("PRD-0002", "Second PRD", PrdStatus::Done, None),
                std::path::PathBuf::from("prds/PRD-0002.md"),
            ),
            (
                "PRD-0003.md".to_string(),
                make_test_prd(
                    "PRD-0003",
                    "Third PRD",
                    PrdStatus::Active,
                    Some(vec!["PRD-0001".to_string(), "PRD-0002".to_string()]),
                ),
                std::path::PathBuf::from("prds/PRD-0003.md"),
            ),
        ];

        let graph = build_graph_from_prds(&prds).unwrap();

        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.edges.len(), 2);

        // Edges should be sorted.
        assert_eq!(graph.edges[0].from, "PRD-0003");
        assert_eq!(graph.edges[0].to, "PRD-0001");
        assert_eq!(graph.edges[1].from, "PRD-0003");
        assert_eq!(graph.edges[1].to, "PRD-0002");
    }

    #[test]
    fn test_build_graph_chain_dependencies() {
        let prds = vec![
            (
                "PRD-0001.md".to_string(),
                make_test_prd("PRD-0001", "First PRD", PrdStatus::Done, None),
                std::path::PathBuf::from("prds/PRD-0001.md"),
            ),
            (
                "PRD-0002.md".to_string(),
                make_test_prd(
                    "PRD-0002",
                    "Second PRD",
                    PrdStatus::Done,
                    Some(vec!["PRD-0001".to_string()]),
                ),
                std::path::PathBuf::from("prds/PRD-0002.md"),
            ),
            (
                "PRD-0003.md".to_string(),
                make_test_prd(
                    "PRD-0003",
                    "Third PRD",
                    PrdStatus::Active,
                    Some(vec!["PRD-0002".to_string()]),
                ),
                std::path::PathBuf::from("prds/PRD-0003.md"),
            ),
        ];

        let graph = build_graph_from_prds(&prds).unwrap();

        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.edges.len(), 2);

        // PRD-0002 -> PRD-0001.
        let edge1 = graph.edges.iter().find(|e| e.from == "PRD-0002").unwrap();
        assert_eq!(edge1.to, "PRD-0001");

        // PRD-0003 -> PRD-0002.
        let edge2 = graph.edges.iter().find(|e| e.from == "PRD-0003").unwrap();
        assert_eq!(edge2.to, "PRD-0002");
    }

    #[test]
    fn test_prd_graph_node_count() {
        let prds = vec![(
            "PRD-0001.md".to_string(),
            make_test_prd(
                "PRD-0001",
                "First PRD",
                PrdStatus::Active,
                Some(vec!["PRD-9999".to_string()]),
            ),
            std::path::PathBuf::from("prds/PRD-0001.md"),
        )];

        let graph = build_graph_from_prds(&prds).unwrap();

        // 2 nodes total (1 real + 1 missing), but node_count excludes missing.
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.node_count(), 1);
    }

    #[test]
    fn test_build_graph_deduplicates_missing_refs() {
        let prds = vec![
            (
                "PRD-0001.md".to_string(),
                make_test_prd(
                    "PRD-0001",
                    "First PRD",
                    PrdStatus::Active,
                    Some(vec!["PRD-9999".to_string()]),
                ),
                std::path::PathBuf::from("prds/PRD-0001.md"),
            ),
            (
                "PRD-0002.md".to_string(),
                make_test_prd(
                    "PRD-0002",
                    "Second PRD",
                    PrdStatus::Active,
                    Some(vec!["PRD-9999".to_string()]),
                ),
                std::path::PathBuf::from("prds/PRD-0002.md"),
            ),
        ];

        let graph = build_graph_from_prds(&prds).unwrap();

        // Should have 3 nodes: PRD-0001, PRD-0002, and one PRD-9999.
        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.edges.len(), 2);

        // Only one missing ref even though referenced twice.
        assert_eq!(graph.missing_refs.len(), 1);
        assert_eq!(graph.missing_refs[0], "PRD-9999");

        // But two warnings.
        assert_eq!(graph.warnings.len(), 2);
    }

    #[test]
    fn test_build_graph_from_actual_repo() {
        // Use the actual .mr/prds directory from this repo.
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));

        let result = build_graph(root);
        assert!(result.is_ok());

        let graph = result.unwrap();

        // We should have at least some PRDs.
        assert!(graph.node_count() > 0);
    }

    // ========================================================================
    // ASCII Rendering Tests
    // ========================================================================

    #[test]
    fn test_render_ascii_empty_graph() {
        let graph = PrdGraph {
            nodes: vec![],
            edges: vec![],
            missing_refs: vec![],
            warnings: vec![],
        };

        let output = render_ascii(&graph, None);

        assert!(output.contains("PRD Dependency Graph"));
        assert!(output.contains("(no PRDs found)"));
    }

    #[test]
    fn test_render_ascii_single_node_no_deps() {
        let prds = vec![(
            "PRD-0001.md".to_string(),
            make_test_prd("PRD-0001", "First PRD", PrdStatus::Active, None),
            std::path::PathBuf::from("prds/PRD-0001.md"),
        )];

        let graph = build_graph_from_prds(&prds).unwrap();
        let output = render_ascii(&graph, None);

        assert!(output.contains("PRD Dependency Graph"));
        assert!(output.contains("[PRD-0001]"));
        assert!(output.contains("First PRD"));
        assert!(output.contains("(active)"));
        assert!(output.contains("1 PRDs, 0 dependencies"));
    }

    #[test]
    fn test_render_ascii_with_dependencies() {
        let prds = vec![
            (
                "PRD-0001.md".to_string(),
                make_test_prd("PRD-0001", "First PRD", PrdStatus::Done, None),
                std::path::PathBuf::from("prds/PRD-0001.md"),
            ),
            (
                "PRD-0002.md".to_string(),
                make_test_prd(
                    "PRD-0002",
                    "Second PRD",
                    PrdStatus::Active,
                    Some(vec!["PRD-0001".to_string()]),
                ),
                std::path::PathBuf::from("prds/PRD-0002.md"),
            ),
        ];

        let graph = build_graph_from_prds(&prds).unwrap();
        let output = render_ascii(&graph, None);

        assert!(output.contains("[PRD-0001]"));
        assert!(output.contains("[PRD-0002]"));
        assert!(output.contains("└── PRD-0001"));
        assert!(output.contains("2 PRDs, 1 dependencies"));
    }

    #[test]
    fn test_render_ascii_with_missing_refs() {
        let prds = vec![(
            "PRD-0001.md".to_string(),
            make_test_prd(
                "PRD-0001",
                "First PRD",
                PrdStatus::Active,
                Some(vec!["PRD-9999".to_string()]),
            ),
            std::path::PathBuf::from("prds/PRD-0001.md"),
        )];

        let graph = build_graph_from_prds(&prds).unwrap();
        let output = render_ascii(&graph, None);

        // Should show the missing reference section.
        assert!(output.contains("--- Missing References ---"));
        assert!(output.contains("- PRD-9999 -"));
        assert!(output.contains("(not found)"));
        assert!(output.contains("Referenced by: PRD-0001"));
        assert!(output.contains("1 missing"));
    }

    #[test]
    fn test_render_ascii_multiple_dependencies() {
        let prds = vec![
            (
                "PRD-0001.md".to_string(),
                make_test_prd("PRD-0001", "First PRD", PrdStatus::Done, None),
                std::path::PathBuf::from("prds/PRD-0001.md"),
            ),
            (
                "PRD-0002.md".to_string(),
                make_test_prd("PRD-0002", "Second PRD", PrdStatus::Done, None),
                std::path::PathBuf::from("prds/PRD-0002.md"),
            ),
            (
                "PRD-0003.md".to_string(),
                make_test_prd(
                    "PRD-0003",
                    "Third PRD",
                    PrdStatus::Active,
                    Some(vec!["PRD-0001".to_string(), "PRD-0002".to_string()]),
                ),
                std::path::PathBuf::from("prds/PRD-0003.md"),
            ),
        ];

        let graph = build_graph_from_prds(&prds).unwrap();
        let output = render_ascii(&graph, None);

        // PRD-0003 should show both dependencies.
        assert!(output.contains("├── PRD-0001"));
        assert!(output.contains("└── PRD-0002"));
        assert!(output.contains("3 PRDs, 2 dependencies"));
    }

    #[test]
    fn test_render_ascii_config_no_titles() {
        let prds = vec![(
            "PRD-0001.md".to_string(),
            make_test_prd("PRD-0001", "First PRD", PrdStatus::Active, None),
            std::path::PathBuf::from("prds/PRD-0001.md"),
        )];

        let graph = build_graph_from_prds(&prds).unwrap();
        let config = AsciiConfig {
            show_titles: false,
            max_title_len: 40,
        };
        let output = render_ascii(&graph, Some(config));

        assert!(output.contains("[PRD-0001]"));
        // Title should not appear between ID and status.
        assert!(output.contains("[PRD-0001] (active)"));
    }

    #[test]
    fn test_render_ascii_truncates_long_titles() {
        let prds = vec![(
            "PRD-0001.md".to_string(),
            make_test_prd(
                "PRD-0001",
                "This is a very long title that should be truncated for display",
                PrdStatus::Active,
                None,
            ),
            std::path::PathBuf::from("prds/PRD-0001.md"),
        )];

        let graph = build_graph_from_prds(&prds).unwrap();
        let config = AsciiConfig {
            show_titles: true,
            max_title_len: 20,
        };
        let output = render_ascii(&graph, Some(config));

        // Title should be truncated with "...".
        assert!(output.contains("This is a very lo..."));
        assert!(!output.contains("truncated for display"));
    }
}
