//! Worktree path resolution and git helpers.
//!
//! Provides utilities for resolving the main worktree path, creating and
//! removing git worktrees, computing modified files, and deriving sibling
//! directory paths following the `../<repo>-prd-<id>/` convention.

// Git module is defined now but consumed by later tasks (T-004 .. T-018).
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

// ── Git command helpers ─────────────────────────────────────────────

/// Run a git command in the given directory and return stdout as a trimmed string.
///
/// Returns an error if the command exits with a non-zero status.
fn git_output(args: &[&str], cwd: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .with_context(|| format!("failed to execute git {}", args.first().unwrap_or(&"")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "git {} failed (exit {}): {}",
            args.first().unwrap_or(&""),
            output.status,
            stderr.trim()
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(stdout)
}

/// Run a git command in the given directory, returning success/failure
/// without capturing output.
fn git_run(args: &[&str], cwd: &Path) -> Result<()> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .with_context(|| format!("failed to execute git {}", args.first().unwrap_or(&"")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "git {} failed (exit {}): {}",
            args.first().unwrap_or(&""),
            output.status,
            stderr.trim()
        );
    }

    Ok(())
}

// ── Path resolution ─────────────────────────────────────────────────

/// Resolve the main worktree (original checkout) root path.
///
/// Uses `git rev-parse --git-common-dir` to find the shared `.git`
/// directory, then derives the main worktree root from it.
///
/// Works correctly from both the main worktree and any linked worktree.
pub fn resolve_main_worktree(cwd: &Path) -> Result<PathBuf> {
    let common_dir = git_output(&["rev-parse", "--git-common-dir"], cwd)
        .context("failed to resolve git common directory")?;

    let common_path = if Path::new(&common_dir).is_absolute() {
        PathBuf::from(&common_dir)
    } else {
        cwd.join(&common_dir)
    };

    // Canonicalize to resolve any `..` segments.
    let canonical = common_path
        .canonicalize()
        .with_context(|| format!("failed to canonicalize git common dir: {common_dir}"))?;

    // The common dir is the `.git` directory of the main worktree.
    // The main worktree root is its parent.
    canonical
        .parent()
        .map(Path::to_path_buf)
        .with_context(|| format!("git common dir has no parent: {}", canonical.display()))
}

/// Resolve the repository name from the main worktree path.
///
/// Extracts the final path component (e.g., `/home/user/projects/microralph` → `microralph`).
pub fn repo_name(main_worktree: &Path) -> Result<String> {
    main_worktree
        .file_name()
        .and_then(|n| n.to_str())
        .map(String::from)
        .with_context(|| {
            format!(
                "failed to extract repo name from path: {}",
                main_worktree.display()
            )
        })
}

/// Derive the branch name for a worktree tied to a PRD.
///
/// Convention: `<repo-name>-prd-<numeric-id>` (e.g., `microralph-prd-39`).
pub fn worktree_branch_name(repo: &str, prd_id: &str) -> String {
    // Strip the "PRD-" prefix and leading zeros to get the numeric portion.
    let numeric = prd_id
        .strip_prefix("PRD-")
        .unwrap_or(prd_id)
        .trim_start_matches('0');

    // Keep at least "0" if the ID was all zeros.
    let numeric = if numeric.is_empty() { "0" } else { numeric };

    format!("{repo}-prd-{numeric}")
}

/// Derive the sibling worktree directory path for a PRD.
///
/// Convention: `../<repo-name>-prd-<numeric-id>/` relative to the main worktree.
pub fn worktree_path(main_worktree: &Path, repo: &str, prd_id: &str) -> PathBuf {
    let dir_name = worktree_branch_name(repo, prd_id);

    main_worktree
        .parent()
        .unwrap_or(main_worktree)
        .join(dir_name)
}

// ── Worktree operations ─────────────────────────────────────────────

/// Create a new git branch from the current HEAD of the given base ref.
///
/// If the branch already exists, this is a no-op.
pub fn create_branch(branch: &str, base: &str, cwd: &Path) -> Result<()> {
    // Check if branch already exists.
    let exists = git_output(&["branch", "--list", branch], cwd)?;

    if !exists.is_empty() {
        return Ok(());
    }

    git_run(&["branch", branch, base], cwd)
        .with_context(|| format!("failed to create branch {branch} from {base}"))
}

/// Create a git worktree at the specified path, checked out to the given branch.
pub fn create_worktree(worktree_path: &Path, branch: &str, cwd: &Path) -> Result<()> {
    let path_str = worktree_path
        .to_str()
        .context("worktree path is not valid UTF-8")?;

    git_run(&["worktree", "add", path_str, branch], cwd)
        .with_context(|| format!("failed to create worktree at {path_str} on branch {branch}"))
}

/// Remove a git worktree.
///
/// Uses `--force` to remove even if there are uncommitted changes.
pub fn remove_worktree(worktree_path: &Path, cwd: &Path) -> Result<()> {
    let path_str = worktree_path
        .to_str()
        .context("worktree path is not valid UTF-8")?;

    git_run(&["worktree", "remove", "--force", path_str], cwd)
        .with_context(|| format!("failed to remove worktree at {path_str}"))
}

/// Delete a git branch.
///
/// Uses `-D` (force delete) to remove even unmerged branches.
pub fn delete_branch(branch: &str, cwd: &Path) -> Result<()> {
    git_run(&["branch", "-D", branch], cwd)
        .with_context(|| format!("failed to delete branch {branch}"))
}

/// List all registered git worktrees and their paths.
///
/// Returns a vec of `(path, branch)` tuples parsed from `git worktree list --porcelain`.
pub fn list_worktrees(cwd: &Path) -> Result<Vec<(PathBuf, Option<String>)>> {
    let output = git_output(&["worktree", "list", "--porcelain"], cwd)?;

    let mut result = Vec::new();
    let mut current_path: Option<PathBuf> = None;
    let mut current_branch: Option<String> = None;

    for line in output.lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            // Flush previous entry.
            if let Some(p) = current_path.take() {
                result.push((p, current_branch.take()));
            }
            current_path = Some(PathBuf::from(path));
        } else if let Some(branch_ref) = line.strip_prefix("branch ") {
            // Extract short branch name from refs/heads/...
            let short = branch_ref.strip_prefix("refs/heads/").unwrap_or(branch_ref);
            current_branch = Some(short.to_string());
        } else if line == "detached" {
            current_branch = None;
        }
    }

    // Flush the last entry.
    if let Some(p) = current_path {
        result.push((p, current_branch));
    }

    Ok(result)
}

// ── Modified files ──────────────────────────────────────────────────

/// Compute files modified in the given branch relative to a target branch.
///
/// Uses `git diff --name-only <target>...<branch>` (three-dot merge-base diff).
pub fn modified_files(branch: &str, target: &str, cwd: &Path) -> Result<Vec<String>> {
    let range = format!("{target}...{branch}");

    let output = git_output(&["diff", "--name-only", &range], cwd)?;

    Ok(output
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect())
}

/// Check if the current working directory is inside a linked worktree
/// (i.e., not the main worktree).
///
/// Compares `--git-dir` with `--git-common-dir` — they differ in linked worktrees.
pub fn is_linked_worktree(cwd: &Path) -> Result<bool> {
    let git_dir = git_output(&["rev-parse", "--git-dir"], cwd)?;
    let common_dir = git_output(&["rev-parse", "--git-common-dir"], cwd)?;

    let git_dir_canon = cwd
        .join(&git_dir)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(&git_dir));

    let common_dir_canon = cwd
        .join(&common_dir)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(&common_dir));

    Ok(git_dir_canon != common_dir_canon)
}

/// Get the current branch name.
pub fn current_branch(cwd: &Path) -> Result<String> {
    git_output(&["rev-parse", "--abbrev-ref", "HEAD"], cwd)
        .context("failed to get current branch name")
}

// ── Merge / rebase helpers ──────────────────────────────────────────

/// Rebase the current branch onto the given target.
///
/// Runs `git rebase <target>` in the given directory.
/// Returns an error if the rebase fails (e.g., due to conflicts).
pub fn rebase_onto(target: &str, cwd: &Path) -> Result<()> {
    git_run(&["rebase", target], cwd).with_context(|| format!("failed to rebase onto {target}"))
}

/// Abort an in-progress rebase.
pub fn rebase_abort(cwd: &Path) -> Result<()> {
    git_run(&["rebase", "--abort"], cwd).context("failed to abort rebase")
}

/// Merge a branch into the current HEAD.
///
/// Uses `--no-edit` to accept the default merge commit message.
pub fn merge_branch(branch: &str, cwd: &Path) -> Result<()> {
    git_run(&["merge", branch, "--no-edit"], cwd)
        .with_context(|| format!("failed to merge branch {branch}"))
}

/// Abort an in-progress merge.
pub fn merge_abort(cwd: &Path) -> Result<()> {
    git_run(&["merge", "--abort"], cwd).context("failed to abort merge")
}

/// Checkout a branch.
pub fn checkout(branch: &str, cwd: &Path) -> Result<()> {
    git_run(&["checkout", branch], cwd)
        .with_context(|| format!("failed to checkout branch {branch}"))
}

/// Attempt to fast-forward merge a branch into the current branch.
///
/// Returns an error if a fast-forward is not possible.
pub fn merge_ff_only(branch: &str, cwd: &Path) -> Result<()> {
    git_run(&["merge", "--ff-only", branch], cwd)
        .with_context(|| format!("failed to fast-forward merge {branch}"))
}

// ── Conflict resolution helpers ─────────────────────────────────────

/// List files with unresolved merge conflicts.
///
/// Runs `git diff --name-only --diff-filter=U` to find unmerged paths.
pub fn list_conflict_files(cwd: &Path) -> Result<Vec<String>> {
    let output = git_output(&["diff", "--name-only", "--diff-filter=U"], cwd)
        .context("failed to list conflict files")?;

    Ok(output
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect())
}

/// Get the full diff showing conflict markers for unresolved files.
///
/// Returns raw `git diff` output, which includes `<<<<<<<`/`=======`/`>>>>>>>`
/// markers for conflicting hunks.
pub fn conflict_diff(cwd: &Path) -> Result<String> {
    git_output(&["diff"], cwd).context("failed to get conflict diff")
}

/// Stage all changes (resolved conflicts and other modifications).
pub fn stage_all(cwd: &Path) -> Result<()> {
    git_run(&["add", "-A"], cwd).context("failed to stage all files")
}

/// Continue a paused rebase after conflicts have been resolved.
///
/// Runs `git -c core.editor=true rebase --continue` to skip editor.
pub fn rebase_continue(cwd: &Path) -> Result<()> {
    git_run(&["-c", "core.editor=true", "rebase", "--continue"], cwd)
        .context("failed to continue rebase")
}

/// Check whether a rebase is currently in progress.
pub fn is_rebase_in_progress(cwd: &Path) -> Result<bool> {
    let git_dir =
        git_output(&["rev-parse", "--git-dir"], cwd).context("failed to resolve git dir")?;
    let git_dir = Path::new(git_dir.trim());

    Ok(git_dir.join("rebase-merge").exists() || git_dir.join("rebase-apply").exists())
}

/// Finalize a merge commit after conflicts have been resolved.
///
/// Stages all changes and commits with `--no-edit` to accept the
/// default merge message.
pub fn merge_commit(cwd: &Path) -> Result<()> {
    git_run(&["-c", "core.editor=true", "commit", "--no-edit"], cwd)
        .context("failed to commit merge")
}

/// Stage a specific file path via `git add <path>`.
pub fn add_file(file_path: &str, cwd: &Path) -> Result<()> {
    git_run(&["add", file_path], cwd).with_context(|| format!("failed to stage file: {file_path}"))
}

/// Commit staged changes with the given message.
pub fn commit(message: &str, cwd: &Path) -> Result<()> {
    git_run(&["commit", "-m", message], cwd).context("failed to commit")
}

/// Check whether there are staged changes ready to commit.
pub fn has_staged_changes(cwd: &Path) -> Result<bool> {
    let result = Command::new("git")
        .args(["diff", "--cached", "--quiet"])
        .current_dir(cwd)
        .output()
        .context("failed to run git diff --cached")?;

    // Exit code 0 = no changes, 1 = changes exist.
    Ok(!result.status.success())
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    /// Helper to initialize a git repo in a temp directory.
    fn init_git_repo(dir: &Path) {
        Command::new("git")
            .args(["init"])
            .current_dir(dir)
            .output()
            .expect("failed to init git repo");
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(dir)
            .output()
            .expect("failed to configure git email");
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(dir)
            .output()
            .expect("failed to configure git name");

        // Need at least one commit for branches/worktrees to work.
        std::fs::write(dir.join("README.md"), "# Test").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(dir)
            .output()
            .expect("failed to stage");
        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(dir)
            .output()
            .expect("failed to commit");
    }

    #[test]
    fn worktree_branch_name_strips_prefix_and_zeros() {
        assert_eq!(worktree_branch_name("myrepo", "PRD-0039"), "myrepo-prd-39");
        assert_eq!(worktree_branch_name("myrepo", "PRD-0001"), "myrepo-prd-1");
        assert_eq!(worktree_branch_name("myrepo", "PRD-0100"), "myrepo-prd-100");
    }

    #[test]
    fn worktree_branch_name_handles_edge_cases() {
        assert_eq!(worktree_branch_name("repo", "PRD-0000"), "repo-prd-0");
        assert_eq!(worktree_branch_name("repo", "0042"), "repo-prd-42");
    }

    #[test]
    fn worktree_path_is_sibling() {
        let main = PathBuf::from("/home/user/projects/microralph");
        let path = worktree_path(&main, "microralph", "PRD-0039");
        assert_eq!(path, PathBuf::from("/home/user/projects/microralph-prd-39"));
    }

    #[test]
    fn repo_name_extracts_final_component() {
        let name = repo_name(Path::new("/home/user/projects/microralph")).unwrap();
        assert_eq!(name, "microralph");
    }

    #[test]
    fn repo_name_handles_trailing_slash() {
        // PathBuf normalizes trailing slashes, so this should still work.
        let path = PathBuf::from("/home/user/projects/myrepo");
        let name = repo_name(&path).unwrap();
        assert_eq!(name, "myrepo");
    }

    #[test]
    fn resolve_main_worktree_from_main() {
        let tmp = tempfile::tempdir().unwrap();
        init_git_repo(tmp.path());

        let main = resolve_main_worktree(tmp.path()).unwrap();

        // Canonicalize both to compare consistently.
        let expected = tmp.path().canonicalize().unwrap();
        assert_eq!(main, expected);
    }

    #[test]
    fn create_and_list_worktree() {
        let tmp = tempfile::tempdir().unwrap();
        let main_dir = tmp.path().join("main-repo");
        std::fs::create_dir(&main_dir).unwrap();
        init_git_repo(&main_dir);

        let wt_path = tmp.path().join("main-repo-prd-42");

        // Create branch and worktree.
        create_branch("main-repo-prd-42", "HEAD", &main_dir).unwrap();
        create_worktree(&wt_path, "main-repo-prd-42", &main_dir).unwrap();

        // Verify worktree exists on disk.
        assert!(wt_path.exists());

        // Verify it appears in git worktree list.
        let worktrees = list_worktrees(&main_dir).unwrap();
        assert!(worktrees.len() >= 2); // main + new worktree

        let wt_entry = worktrees
            .iter()
            .find(|(_, b)| b.as_deref() == Some("main-repo-prd-42"));
        assert!(wt_entry.is_some());
    }

    #[test]
    fn create_branch_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        init_git_repo(tmp.path());

        create_branch("test-branch", "HEAD", tmp.path()).unwrap();
        create_branch("test-branch", "HEAD", tmp.path()).unwrap();

        // Should not error on second call.
        let branches = git_output(&["branch", "--list", "test-branch"], tmp.path()).unwrap();
        assert!(!branches.is_empty());
    }

    #[test]
    fn remove_worktree_cleans_up() {
        let tmp = tempfile::tempdir().unwrap();
        let main_dir = tmp.path().join("main-repo");
        std::fs::create_dir(&main_dir).unwrap();
        init_git_repo(&main_dir);

        let wt_path = tmp.path().join("main-repo-prd-99");

        create_branch("main-repo-prd-99", "HEAD", &main_dir).unwrap();
        create_worktree(&wt_path, "main-repo-prd-99", &main_dir).unwrap();
        assert!(wt_path.exists());

        remove_worktree(&wt_path, &main_dir).unwrap();
        assert!(!wt_path.exists());
    }

    #[test]
    fn modified_files_detects_changes() {
        let tmp = tempfile::tempdir().unwrap();
        init_git_repo(tmp.path());

        // Create a feature branch and modify a file.
        git_run(&["checkout", "-b", "feature"], tmp.path()).unwrap();
        std::fs::write(tmp.path().join("new_file.rs"), "fn main() {}").unwrap();
        git_run(&["add", "."], tmp.path()).unwrap();
        git_run(&["commit", "-m", "Add new file"], tmp.path()).unwrap();

        // Get the default branch name (could be "main" or "master").
        git_run(&["checkout", "-"], tmp.path()).unwrap();
        let default_branch = current_branch(tmp.path()).unwrap();

        let files = modified_files("feature", &default_branch, tmp.path()).unwrap();
        assert!(files.contains(&"new_file.rs".to_string()));
    }

    #[test]
    fn modified_files_empty_when_same() {
        let tmp = tempfile::tempdir().unwrap();
        init_git_repo(tmp.path());

        let branch = current_branch(tmp.path()).unwrap();
        let files = modified_files(&branch, &branch, tmp.path()).unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn is_linked_worktree_false_for_main() {
        let tmp = tempfile::tempdir().unwrap();
        init_git_repo(tmp.path());

        let linked = is_linked_worktree(tmp.path()).unwrap();
        assert!(!linked);
    }

    #[test]
    fn is_linked_worktree_true_for_worktree() {
        let tmp = tempfile::tempdir().unwrap();
        let main_dir = tmp.path().join("main-repo");
        std::fs::create_dir(&main_dir).unwrap();
        init_git_repo(&main_dir);

        let wt_path = tmp.path().join("main-repo-prd-7");
        create_branch("main-repo-prd-7", "HEAD", &main_dir).unwrap();
        create_worktree(&wt_path, "main-repo-prd-7", &main_dir).unwrap();

        let linked = is_linked_worktree(&wt_path).unwrap();
        assert!(linked);
    }

    #[test]
    fn resolve_main_worktree_from_linked() {
        let tmp = tempfile::tempdir().unwrap();
        let main_dir = tmp.path().join("main-repo");
        std::fs::create_dir(&main_dir).unwrap();
        init_git_repo(&main_dir);

        let wt_path = tmp.path().join("main-repo-prd-8");
        create_branch("main-repo-prd-8", "HEAD", &main_dir).unwrap();
        create_worktree(&wt_path, "main-repo-prd-8", &main_dir).unwrap();

        // From within the linked worktree, resolve main should return the main repo.
        let resolved = resolve_main_worktree(&wt_path).unwrap();
        let expected = main_dir.canonicalize().unwrap();
        assert_eq!(resolved, expected);
    }

    #[test]
    fn current_branch_returns_name() {
        let tmp = tempfile::tempdir().unwrap();
        init_git_repo(tmp.path());

        let branch = current_branch(tmp.path()).unwrap();
        // Git default branch could be "main" or "master".
        assert!(!branch.is_empty());
    }

    #[test]
    fn delete_branch_removes_branch() {
        let tmp = tempfile::tempdir().unwrap();
        init_git_repo(tmp.path());

        create_branch("to-delete", "HEAD", tmp.path()).unwrap();
        delete_branch("to-delete", tmp.path()).unwrap();

        let list = git_output(&["branch", "--list", "to-delete"], tmp.path()).unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn git_output_returns_error_on_bad_command() {
        let tmp = tempfile::tempdir().unwrap();
        init_git_repo(tmp.path());

        let result = git_output(&["log", "--nonexistent-flag-xyz"], tmp.path());
        assert!(result.is_err());
    }

    #[test]
    fn checkout_switches_branch() {
        let tmp = tempfile::tempdir().unwrap();
        init_git_repo(tmp.path());

        create_branch("test-checkout", "HEAD", tmp.path()).unwrap();
        checkout("test-checkout", tmp.path()).unwrap();

        let branch = current_branch(tmp.path()).unwrap();
        assert_eq!(branch, "test-checkout");
    }

    #[test]
    fn rebase_onto_succeeds_with_no_conflicts() {
        let tmp = tempfile::tempdir().unwrap();
        init_git_repo(tmp.path());

        let default_branch = current_branch(tmp.path()).unwrap();

        // Create a feature branch with a new file.
        git_run(&["checkout", "-b", "feature-rebase"], tmp.path()).unwrap();
        std::fs::write(tmp.path().join("feature.rs"), "fn feature() {}").unwrap();
        git_run(&["add", "."], tmp.path()).unwrap();
        git_run(&["commit", "-m", "Feature commit"], tmp.path()).unwrap();

        // Rebase onto default branch (no-op since no divergence).
        rebase_onto(&default_branch, tmp.path()).unwrap();
    }

    #[test]
    fn rebase_onto_fails_with_conflicts_and_abort_recovers() {
        let tmp = tempfile::tempdir().unwrap();
        init_git_repo(tmp.path());

        let default_branch = current_branch(tmp.path()).unwrap();

        // Create conflicting changes on default branch.
        std::fs::write(tmp.path().join("README.md"), "main changes").unwrap();
        git_run(&["add", "."], tmp.path()).unwrap();
        git_run(&["commit", "-m", "Main change"], tmp.path()).unwrap();

        // Create a feature branch from the original commit with conflicting changes.
        git_run(
            &["checkout", "-b", "feature-conflict", "HEAD~1"],
            tmp.path(),
        )
        .unwrap();
        std::fs::write(tmp.path().join("README.md"), "feature changes").unwrap();
        git_run(&["add", "."], tmp.path()).unwrap();
        git_run(&["commit", "-m", "Feature change"], tmp.path()).unwrap();

        // Rebase should fail due to conflict.
        let result = rebase_onto(&default_branch, tmp.path());
        assert!(result.is_err());

        // Abort should recover.
        rebase_abort(tmp.path()).unwrap();
    }

    #[test]
    fn merge_branch_succeeds() {
        let tmp = tempfile::tempdir().unwrap();
        init_git_repo(tmp.path());

        let default_branch = current_branch(tmp.path()).unwrap();

        // Create a feature branch with changes.
        git_run(&["checkout", "-b", "feature-merge"], tmp.path()).unwrap();
        std::fs::write(tmp.path().join("merge.rs"), "fn merge() {}").unwrap();
        git_run(&["add", "."], tmp.path()).unwrap();
        git_run(&["commit", "-m", "Merge commit"], tmp.path()).unwrap();

        // Go back to default branch and merge feature.
        checkout(&default_branch, tmp.path()).unwrap();
        merge_branch("feature-merge", tmp.path()).unwrap();

        // File should now exist on the default branch.
        assert!(tmp.path().join("merge.rs").exists());
    }

    #[test]
    fn merge_ff_only_succeeds_when_fast_forward() {
        let tmp = tempfile::tempdir().unwrap();
        init_git_repo(tmp.path());

        let default_branch = current_branch(tmp.path()).unwrap();

        // Create a branch ahead of default.
        git_run(&["checkout", "-b", "ahead"], tmp.path()).unwrap();
        std::fs::write(tmp.path().join("ahead.rs"), "fn ahead() {}").unwrap();
        git_run(&["add", "."], tmp.path()).unwrap();
        git_run(&["commit", "-m", "Ahead commit"], tmp.path()).unwrap();

        // Go back and fast-forward.
        checkout(&default_branch, tmp.path()).unwrap();
        merge_ff_only("ahead", tmp.path()).unwrap();
    }

    #[test]
    fn merge_ff_only_fails_when_diverged() {
        let tmp = tempfile::tempdir().unwrap();
        init_git_repo(tmp.path());

        let default_branch = current_branch(tmp.path()).unwrap();

        // Create diverging changes.
        git_run(&["checkout", "-b", "diverged"], tmp.path()).unwrap();
        std::fs::write(tmp.path().join("diverged.rs"), "fn diverged() {}").unwrap();
        git_run(&["add", "."], tmp.path()).unwrap();
        git_run(&["commit", "-m", "Diverged commit"], tmp.path()).unwrap();

        checkout(&default_branch, tmp.path()).unwrap();
        std::fs::write(tmp.path().join("main_change.rs"), "fn main_change() {}").unwrap();
        git_run(&["add", "."], tmp.path()).unwrap();
        git_run(&["commit", "-m", "Main diverge"], tmp.path()).unwrap();

        // Fast-forward should fail.
        let result = merge_ff_only("diverged", tmp.path());
        assert!(result.is_err());
    }

    #[test]
    fn list_conflict_files_returns_conflicting_paths() {
        let tmp = tempfile::tempdir().unwrap();
        init_git_repo(tmp.path());

        let default_branch = current_branch(tmp.path()).unwrap();

        // Create a branch with conflicting changes.
        git_run(&["checkout", "-b", "conflict-br"], tmp.path()).unwrap();
        std::fs::write(tmp.path().join("README.md"), "# Branch version").unwrap();
        git_run(&["add", "."], tmp.path()).unwrap();
        git_run(&["commit", "-m", "Branch change"], tmp.path()).unwrap();

        checkout(&default_branch, tmp.path()).unwrap();
        std::fs::write(tmp.path().join("README.md"), "# Main version").unwrap();
        git_run(&["add", "."], tmp.path()).unwrap();
        git_run(&["commit", "-m", "Main change"], tmp.path()).unwrap();

        // Start a merge (will produce conflicts).
        let _ = merge_branch("conflict-br", tmp.path());

        let files = list_conflict_files(tmp.path()).unwrap();
        assert_eq!(files, vec!["README.md"]);

        // Conflict diff should contain markers.
        let diff = conflict_diff(tmp.path()).unwrap();
        assert!(diff.contains("README.md") || diff.contains("<<<<<<<") || diff.contains("======="));

        // Abort to clean up.
        let _ = merge_abort(tmp.path());
    }

    #[test]
    fn is_rebase_in_progress_false_normally() {
        let tmp = tempfile::tempdir().unwrap();
        init_git_repo(tmp.path());

        assert!(!is_rebase_in_progress(tmp.path()).unwrap());
    }

    #[test]
    fn stage_all_stages_new_files() {
        let tmp = tempfile::tempdir().unwrap();
        init_git_repo(tmp.path());

        std::fs::write(tmp.path().join("new.txt"), "hello").unwrap();
        stage_all(tmp.path()).unwrap();

        // Verify the file is staged.
        let status = git_output(&["status", "--porcelain"], tmp.path()).unwrap();
        assert!(status.contains("new.txt"));
    }

    #[test]
    fn add_file_stages_specific_file() {
        let tmp = tempfile::tempdir().unwrap();
        init_git_repo(tmp.path());

        std::fs::write(tmp.path().join("a.txt"), "aaa").unwrap();
        std::fs::write(tmp.path().join("b.txt"), "bbb").unwrap();

        add_file("a.txt", tmp.path()).unwrap();

        let status = git_output(&["status", "--porcelain"], tmp.path()).unwrap();
        assert!(status.contains("A  a.txt"));
        assert!(status.contains("?? b.txt"));
    }

    #[test]
    fn commit_creates_commit_with_message() {
        let tmp = tempfile::tempdir().unwrap();
        init_git_repo(tmp.path());

        std::fs::write(tmp.path().join("file.txt"), "content").unwrap();
        stage_all(tmp.path()).unwrap();
        commit("test commit message", tmp.path()).unwrap();

        let log = git_output(&["log", "--oneline", "-1"], tmp.path()).unwrap();
        assert!(log.contains("test commit message"));
    }

    #[test]
    fn has_staged_changes_detects_changes() {
        let tmp = tempfile::tempdir().unwrap();
        init_git_repo(tmp.path());

        // No staged changes initially.
        assert!(!has_staged_changes(tmp.path()).unwrap());

        std::fs::write(tmp.path().join("new.txt"), "content").unwrap();
        stage_all(tmp.path()).unwrap();

        // Now there are staged changes.
        assert!(has_staged_changes(tmp.path()).unwrap());
    }
}
