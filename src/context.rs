use chrono::Local;
use std::env;

// ── Public types ──────────────────────────────────────────────────────────────

pub struct NoteContext {
    pub timestamp: String,
    pub directory: String,
    pub git_repo: String,
    pub git_branch: String,
    /// Paths of staged files relative to the repo root. Empty outside git repos
    /// or when nothing is staged.
    pub changed_files: Vec<String>,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Capture the current environment. Always succeeds — git failures fall back gracefully.
pub fn capture_context() -> NoteContext {
    let directory = env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .to_string_lossy()
        .into_owned();

    let timestamp = Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();

    let (git_repo, git_branch, changed_files) = capture_git_context(&directory)
        .unwrap_or_else(|_| ("none".to_string(), "none".to_string(), Vec::new()));

    NoteContext {
        timestamp,
        directory,
        git_repo,
        git_branch,
        changed_files,
    }
}

// ── Git context ───────────────────────────────────────────────────────────────

fn capture_git_context(dir: &str) -> Result<(String, String, Vec<String>), git2::Error> {
    let repo = git2::Repository::discover(dir)?;
    let branch = get_branch(&repo);
    let repo_name = get_repo_name(&repo);
    let staged = get_staged_files(&repo);
    Ok((repo_name, branch, staged))
}

fn get_branch(repo: &git2::Repository) -> String {
    match repo.head() {
        Ok(head) if head.is_branch() => head.shorthand().unwrap_or("none").to_string(),
        _ => "none".to_string(), // detached HEAD or error
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
