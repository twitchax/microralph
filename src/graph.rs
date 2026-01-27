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
}
