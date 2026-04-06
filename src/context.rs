use chrono::Local;
use std::env;

// ── Public types ──────────────────────────────────────────────────────────────

pub struct NoteContext {
    pub timestamp: String,
    pub directory: String,
    pub git_repo: String,
    pub git_branch: String,
    /// Current commit hash (40-char SHA-1) or "none" if not in a git repo or detached.
    pub commit_hash: String,
    /// Staged files (index vs HEAD). Empty outside git repos or when nothing is staged.
    pub changed_files: Vec<String>,
    /// Tracked files modified in the working tree but not staged (index vs workdir).
    pub unstaged_files: Vec<String>,
    /// New files not yet known to git (untracked).
    pub untracked_files: Vec<String>,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Capture the current environment. Always succeeds — git failures fall back gracefully.
pub fn capture_context() -> NoteContext {
    let directory = env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .to_string_lossy()
        .into_owned();

    let timestamp = Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();

    let (git_repo, git_branch, commit_hash, changed_files, unstaged_files, untracked_files) =
        capture_git_context(&directory).unwrap_or_else(|_| {
            (
                "none".to_string(),
                "none".to_string(),
                "none".to_string(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            )
        });

    NoteContext {
        timestamp,
        directory,
        git_repo,
        git_branch,
        commit_hash,
        changed_files,
        unstaged_files,
        untracked_files,
    }
}

// ── Git context ───────────────────────────────────────────────────────────────

/// (repo_name, branch, commit_hash, staged_files, unstaged_files, untracked_files)
type GitContext = (
    String,
    String,
    String,
    Vec<String>,
    Vec<String>,
    Vec<String>,
);

fn capture_git_context(dir: &str) -> Result<GitContext, git2::Error> {
    let repo = git2::Repository::discover(dir)?;
    let branch = get_branch(&repo);
    let repo_name = get_repo_name(&repo);
    let commit_hash = get_commit_hash(&repo);
    let staged = get_staged_files(&repo);
    let unstaged = get_unstaged_files(&repo);
    let untracked = get_untracked_files(&repo);
    Ok((repo_name, branch, commit_hash, staged, unstaged, untracked))
}

fn get_branch(repo: &git2::Repository) -> String {
    match repo.head() {
        Ok(head) if head.is_branch() => head.shorthand().unwrap_or("none").to_string(),
        _ => "none".to_string(), // detached HEAD or error
    }
}

/// Returns the current HEAD commit as a 40-char hex SHA-1, or "none" if repo is empty/error.
fn get_commit_hash(repo: &git2::Repository) -> String {
    match repo.head() {
        Ok(head) => match head.peel_to_commit() {
            Ok(commit) => commit.id().to_string(),
            Err(_) => "none".to_string(), // Empty repo (no commits yet)
        },
        Err(_) => "none".to_string(), // Detached HEAD or other error
    }
}

/// Returns paths of staged files relative to the repo root.
///
/// Uses the same approach as `git diff --cached --name-only`: compares HEAD
/// tree against the index (staging area). On a repo with no commits yet we
/// pass `None` as the old tree, which diffs against an empty tree.
fn get_staged_files(repo: &git2::Repository) -> Vec<String> {
    let head_tree = repo.head().ok().and_then(|h| h.peel_to_tree().ok());

    let mut diff_opts = git2::DiffOptions::new();
    diff_opts.include_untracked(false);

    let diff = match repo.diff_tree_to_index(
        head_tree.as_ref(),
        None, // None = use repo's index
        Some(&mut diff_opts),
    ) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };

    let mut files: Vec<String> = Vec::new();
    let _ = diff.foreach(
        &mut |delta: git2::DiffDelta<'_>, _| {
            // Use new_file path (handles renames correctly)
            if let Some(path) = delta.new_file().path() {
                files.push(path.to_string_lossy().into_owned());
            }
            true
        },
        None,
        None,
        None,
    );

    files.sort();
    files
}

/// Returns paths of modified/deleted files in the working tree that are NOT staged
/// (equivalent to `git diff --name-only`): compares the index against the working directory.
fn get_unstaged_files(repo: &git2::Repository) -> Vec<String> {
    let mut diff_opts = git2::DiffOptions::new();
    diff_opts.include_untracked(false);

    let diff = match repo.diff_index_to_workdir(None, Some(&mut diff_opts)) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };

    let mut files: Vec<String> = Vec::new();
    let _ = diff.foreach(
        &mut |delta: git2::DiffDelta<'_>, _| {
            if let Some(path) = delta.new_file().path() {
                files.push(path.to_string_lossy().into_owned());
            }
            true
        },
        None,
        None,
        None,
    );

    files.sort();
    files
}

/// Returns paths of files git does not know about at all (untracked).
/// Equivalent to the "Untracked files" section of `git status`.
fn get_untracked_files(repo: &git2::Repository) -> Vec<String> {
    let mut diff_opts = git2::DiffOptions::new();
    diff_opts.include_untracked(true);
    diff_opts.recurse_untracked_dirs(true);

    // diff_index_to_workdir with include_untracked gives us both modified and
    // untracked deltas. Filter to only UNTRACKED status.
    let diff = match repo.diff_index_to_workdir(None, Some(&mut diff_opts)) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };

    let mut files: Vec<String> = Vec::new();
    let _ = diff.foreach(
        &mut |delta: git2::DiffDelta<'_>, _| {
            if delta.status() == git2::Delta::Untracked {
                if let Some(path) = delta.new_file().path() {
                    files.push(path.to_string_lossy().into_owned());
                }
            }
            true
        },
        None,
        None,
        None,
    );

    files.sort();
    files
}

fn get_repo_name(repo: &git2::Repository) -> String {
    if let Ok(remotes) = repo.remotes() {
        // Collect remote names, filtering out non-UTF-8 entries
        let remote_names: Vec<String> = remotes.iter().flatten().map(String::from).collect();

        // Prefer "origin"; otherwise use first remote alphabetically
        let target = if remote_names.iter().any(|r| r == "origin") {
            Some("origin".to_string())
        } else {
            let mut sorted = remote_names.clone();
            sorted.sort();
            sorted.into_iter().next()
        };

        if let Some(name) = target {
            if let Ok(remote) = repo.find_remote(&name) {
                if let Some(url) = remote.url() {
                    if let Some(parsed) = parse_repo_name_from_url(url) {
                        return parsed;
                    }
                }
            }
        }
    }

    // Fallback: root directory name of the git repo
    repo.workdir()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("none")
        .to_string()
}

pub(crate) fn parse_repo_name_from_url(url: &str) -> Option<String> {
    // SSH:   git@github.com:user/repo.git
    // HTTPS: https://github.com/user/repo.git
    let segment = if url.starts_with("git@") || (!url.starts_with("http") && url.contains(':')) {
        // SSH: take everything after the colon, then the last path segment
        url.split_once(':')?
            .1
            .trim_end_matches('/')
            .split('/')
            .next_back()?
    } else {
        // HTTPS: take the last path segment
        url.trim_end_matches('/').split('/').next_back()?
    };

    let name = segment.trim_end_matches(".git");
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::parse_repo_name_from_url;

    #[test]
    fn https_url_with_git_suffix() {
        assert_eq!(
            parse_repo_name_from_url("https://github.com/user/my-app.git"),
            Some("my-app".to_string())
        );
    }

    #[test]
    fn https_url_without_git_suffix() {
        assert_eq!(
            parse_repo_name_from_url("https://github.com/user/my-app"),
            Some("my-app".to_string())
        );
    }

    #[test]
    fn ssh_url_with_git_suffix() {
        assert_eq!(
            parse_repo_name_from_url("git@github.com:user/my-app.git"),
            Some("my-app".to_string())
        );
    }

    #[test]
    fn ssh_url_without_git_suffix() {
        assert_eq!(
            parse_repo_name_from_url("git@github.com:user/my-app"),
            Some("my-app".to_string())
        );
    }

    #[test]
    fn https_url_trailing_slash() {
        assert_eq!(
            parse_repo_name_from_url("https://github.com/user/my-app/"),
            Some("my-app".to_string())
        );
    }

    #[test]
    fn https_url_no_path_segment() {
        // After trim_end_matches('/') → "https://github.com", split('/').next_back() → "github.com"
        // Not empty, so Some("github.com") — acceptable fallback
        assert!(parse_repo_name_from_url("https://github.com/").is_some());
    }

    #[test]
    fn empty_string() {
        assert_eq!(parse_repo_name_from_url(""), None);
    }
}
