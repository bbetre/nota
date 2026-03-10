use chrono::Local;
use std::env;

// ── Public types ──────────────────────────────────────────────────────────────

pub struct NoteContext {
    pub timestamp: String,
    pub directory: String,
    pub git_repo: String,
    pub git_branch: String,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Capture the current environment. Always succeeds — git failures fall back to "none".
pub fn capture_context() -> NoteContext {
    let directory = env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .to_string_lossy()
        .into_owned();

    let timestamp = Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();

    let (git_repo, git_branch) = capture_git_context(&directory)
        .unwrap_or_else(|_| ("none".to_string(), "none".to_string()));

    NoteContext {
        timestamp,
        directory,
        git_repo,
        git_branch,
    }
}

// ── Git context ───────────────────────────────────────────────────────────────

fn capture_git_context(dir: &str) -> Result<(String, String), git2::Error> {
    let repo = git2::Repository::discover(dir)?;
    let branch = get_branch(&repo);
    let repo_name = get_repo_name(&repo);
    Ok((repo_name, branch))
}

fn get_branch(repo: &git2::Repository) -> String {
    match repo.head() {
        Ok(head) if head.is_branch() => head.shorthand().unwrap_or("none").to_string(),
        _ => "none".to_string(), // detached HEAD or error
    }
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
        // Edge case: bare domain — should return None or the domain
        let result = parse_repo_name_from_url("https://github.com/");
        // After trim_end_matches('/') → "https://github.com", split('/').last() → "github.com"
        // Not empty, so Some("github.com") — acceptable fallback
        assert!(result.is_some());
    }

    #[test]
    fn empty_string() {
        assert_eq!(parse_repo_name_from_url(""), None);
    }
}
