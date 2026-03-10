use crate::note::{self, Note};
use chrono::{Datelike, Duration};
use std::collections::HashMap;
use std::path::Path;

// ── Structs ───────────────────────────────────────────────────────────────────

pub struct FilterOptions {
    pub limit: usize,
    pub here: bool,
    pub repo: Option<String>,
    pub branch: Option<String>,
    pub tag: Option<String>,
    pub current_dir: Option<String>,
    pub current_repo: Option<String>,
}

impl Default for FilterOptions {
    fn default() -> Self {
        Self {
            limit: 20,
            here: false,
            repo: None,
            branch: None,
            tag: None,
            current_dir: None,
            current_repo: None,
        }
    }
}

pub struct RepoGroup {
    pub repo: String,
    pub note_count: usize,
    pub last_activity: String,
}

// ── Stats ─────────────────────────────────────────────────────────────────────

pub struct NoteStats {
    pub total: usize,
    pub today: usize,
    pub this_week: usize,
    pub this_month: usize,
    pub most_active_repo: Option<String>,
    pub most_active_repo_count: usize,
}

// ── Loading ───────────────────────────────────────────────────────────────────

pub fn load_all_notes(notes_dir: &Path) -> Vec<Note> {
    if !notes_dir.exists() {
        return Vec::new();
    }

    walkdir::WalkDir::new(notes_dir)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file() && e.path().extension().and_then(|s| s.to_str()) == Some("md")
        })
        .filter_map(|e| {
            let path = e.path().to_path_buf();
            let content = std::fs::read_to_string(&path).ok()?;
            match note::parse_note_file(&content, &path) {
                Ok(n) => Some(n),
                Err(_) => {
                    eprintln!(
                        "warn: skipping malformed note {}",
                        path.file_name().unwrap_or_default().to_string_lossy()
                    );
                    None
                }
            }
        })
        .collect()
}

// ── Sorting ───────────────────────────────────────────────────────────────────

pub fn sort_by_recency(notes: &mut [Note]) {
    notes.sort_by(|a, b| {
        let ta = note::parse_timestamp(&a.frontmatter.timestamp);
        let tb = note::parse_timestamp(&b.frontmatter.timestamp);
        // Most recent first; None timestamps sort last (None < Some)
        tb.cmp(&ta)
    });
}

// ── Filtering ─────────────────────────────────────────────────────────────────

pub fn apply_filters(notes: Vec<Note>, opts: &FilterOptions) -> Vec<Note> {
    notes
        .into_iter()
        .filter(|n| {
            // --here: both repo AND directory must match
            if opts.here {
                let repo_match = opts
                    .current_repo
                    .as_deref()
                    .map(|r| n.frontmatter.git_repo == r)
                    .unwrap_or(false);
                let dir_match = opts
                    .current_dir
                    .as_deref()
                    .map(|d| n.frontmatter.directory == d)
                    .unwrap_or(false);
                if !(repo_match && dir_match) {
                    return false;
                }
            }
            // --repo
            if let Some(repo) = &opts.repo {
                if &n.frontmatter.git_repo != repo {
                    return false;
                }
            }
            // --branch
            if let Some(branch) = &opts.branch {
                if &n.frontmatter.git_branch != branch {
                    return false;
                }
            }
            // --tag
            if let Some(tag) = &opts.tag {
                let tag_lower = tag.to_lowercase();
                if !n
                    .frontmatter
                    .tags
                    .iter()
                    .any(|t| t.to_lowercase() == tag_lower)
                {
                    return false;
                }
            }
            true
        })
        .take(opts.limit)
        .collect()
}

// ── Search ────────────────────────────────────────────────────────────────────

pub fn search_notes(notes: Vec<Note>, query: &str) -> Vec<Note> {
    let query_lower = query.to_lowercase();
    notes
        .into_iter()
        .filter(|n| n.body.to_lowercase().contains(&query_lower))
        .collect()
}

// ── Log grouping ──────────────────────────────────────────────────────────────

pub fn group_by_repo(notes: &[Note], days: Option<u64>) -> Vec<RepoGroup> {
    let cutoff = days.map(|d| chrono::Local::now().naive_local() - Duration::days(d as i64));

    let filtered: Vec<&Note> = notes
        .iter()
        .filter(|n| {
            if let Some(cutoff_dt) = cutoff {
                note::parse_timestamp(&n.frontmatter.timestamp)
                    .map(|ts| ts >= cutoff_dt)
                    .unwrap_or(false)
            } else {
                true
            }
        })
        .collect();

    // Group by repo: track (count, max_timestamp)
    let mut map: HashMap<String, (usize, Option<chrono::NaiveDateTime>)> = HashMap::new();
    for n in &filtered {
        let ts = note::parse_timestamp(&n.frontmatter.timestamp);
        let entry = map
            .entry(n.frontmatter.git_repo.clone())
            .or_insert((0, None));
        entry.0 += 1;
        if ts > entry.1 {
            entry.1 = ts;
        }
    }

    let mut groups: Vec<RepoGroup> = map
        .into_iter()
        .map(|(repo, (count, last_ts))| RepoGroup {
            repo,
            note_count: count,
            last_activity: last_ts
                .map(|ts| ts.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "unknown".to_string()),
        })
        .collect();

    // Sort by most recent activity descending
    groups.sort_by(|a, b| b.last_activity.cmp(&a.last_activity));
    groups
}

// ── Stats ─────────────────────────────────────────────────────────────────────

pub fn compute_stats(notes: &[Note]) -> NoteStats {
    let now = chrono::Local::now().naive_local();
    let today_start = now.date().and_hms_opt(0, 0, 0).unwrap();
    let week_start = today_start - Duration::days(now.weekday().num_days_from_monday() as i64);
    let month_start = chrono::NaiveDate::from_ymd_opt(now.year(), now.month(), 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();

    let mut today = 0usize;
    let mut this_week = 0usize;
    let mut this_month = 0usize;
    let mut repo_counts: HashMap<String, usize> = HashMap::new();

    for n in notes {
        if let Some(ts) = note::parse_timestamp(&n.frontmatter.timestamp) {
            if ts >= today_start {
                today += 1;
            }
            if ts >= week_start {
                this_week += 1;
            }
            if ts >= month_start {
                this_month += 1;
            }
        }
        if n.frontmatter.git_repo != "none" {
            *repo_counts
                .entry(n.frontmatter.git_repo.clone())
                .or_insert(0) += 1;
        }
    }

    let (most_active_repo, most_active_repo_count) = repo_counts
        .into_iter()
        .max_by_key(|(_, c)| *c)
        .map(|(r, c)| (Some(r), c))
        .unwrap_or((None, 0));

    NoteStats {
        total: notes.len(),
        today,
        this_week,
        this_month,
        most_active_repo,
        most_active_repo_count,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::note::{Note, NoteFrontmatter};
    use std::path::PathBuf;

    fn make_note(
        id: &str,
        repo: &str,
        branch: &str,
        dir: &str,
        ts: &str,
        tags: Vec<&str>,
        body: &str,
    ) -> Note {
        Note {
            frontmatter: NoteFrontmatter {
                id: id.to_string(),
                timestamp: ts.to_string(),
                directory: dir.to_string(),
                git_repo: repo.to_string(),
                git_branch: branch.to_string(),
                tags: tags.into_iter().map(String::from).collect(),
                changed_files: Vec::new(),
            },
            body: body.to_string(),
            file_path: PathBuf::from(format!("/tmp/{}.md", id)),
        }
    }

    #[test]
    fn sort_by_recency_orders_most_recent_first() {
        let mut notes = vec![
            make_note("a", "r", "m", "/d", "2026-01-01T10:00:00", vec![], "old"),
            make_note("b", "r", "m", "/d", "2026-03-01T10:00:00", vec![], "new"),
            make_note("c", "r", "m", "/d", "2026-02-01T10:00:00", vec![], "mid"),
        ];
        sort_by_recency(&mut notes);
        assert_eq!(notes[0].frontmatter.id, "b");
        assert_eq!(notes[1].frontmatter.id, "c");
        assert_eq!(notes[2].frontmatter.id, "a");
    }

    #[test]
    fn apply_filters_repo() {
        let notes = vec![
            make_note(
                "a",
                "alpha",
                "main",
                "/d",
                "2026-01-01T00:00:00",
                vec![],
                "",
            ),
            make_note("b", "beta", "main", "/d", "2026-01-01T00:00:00", vec![], ""),
        ];
        let opts = FilterOptions {
            repo: Some("alpha".into()),
            ..Default::default()
        };
        let result = apply_filters(notes, &opts);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].frontmatter.id, "a");
    }

    #[test]
    fn apply_filters_branch() {
        let notes = vec![
            make_note("a", "r", "main", "/d", "2026-01-01T00:00:00", vec![], ""),
            make_note("b", "r", "feature", "/d", "2026-01-01T00:00:00", vec![], ""),
        ];
        let opts = FilterOptions {
            branch: Some("feature".into()),
            ..Default::default()
        };
        let result = apply_filters(notes, &opts);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].frontmatter.id, "b");
    }

    #[test]
    fn apply_filters_tag_case_insensitive() {
        let notes = vec![
            make_note("a", "r", "m", "/d", "2026-01-01T00:00:00", vec!["Rust"], ""),
            make_note("b", "r", "m", "/d", "2026-01-01T00:00:00", vec!["cli"], ""),
        ];
        let opts = FilterOptions {
            tag: Some("rust".into()),
            ..Default::default()
        };
        let result = apply_filters(notes, &opts);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].frontmatter.id, "a");
    }

    #[test]
    fn apply_filters_here() {
        let notes = vec![
            make_note(
                "a",
                "myrepo",
                "m",
                "/projects/myrepo",
                "2026-01-01T00:00:00",
                vec![],
                "",
            ),
            make_note(
                "b",
                "myrepo",
                "m",
                "/other/dir",
                "2026-01-01T00:00:00",
                vec![],
                "",
            ),
            make_note(
                "c",
                "other",
                "m",
                "/projects/myrepo",
                "2026-01-01T00:00:00",
                vec![],
                "",
            ),
        ];
        let opts = FilterOptions {
            here: true,
            current_repo: Some("myrepo".into()),
            current_dir: Some("/projects/myrepo".into()),
            ..Default::default()
        };
        let result = apply_filters(notes, &opts);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].frontmatter.id, "a");
    }

    #[test]
    fn apply_filters_limit() {
        let notes = (0..10)
            .map(|i| {
                make_note(
                    &format!("{:08}", i),
                    "r",
                    "m",
                    "/d",
                    "2026-01-01T00:00:00",
                    vec![],
                    "",
                )
            })
            .collect();
        let opts = FilterOptions {
            limit: 3,
            ..Default::default()
        };
        let result = apply_filters(notes, &opts);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn search_notes_case_insensitive() {
        let notes = vec![
            make_note(
                "a",
                "r",
                "m",
                "/d",
                "2026-01-01T00:00:00",
                vec![],
                "Check the Auth middleware",
            ),
            make_note(
                "b",
                "r",
                "m",
                "/d",
                "2026-01-01T00:00:00",
                vec![],
                "unrelated note",
            ),
        ];
        let result = search_notes(notes, "auth");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].frontmatter.id, "a");
    }

    #[test]
    fn group_by_repo_counts() {
        let notes = vec![
            make_note("a", "alpha", "m", "/d", "2026-03-01T00:00:00", vec![], ""),
            make_note("b", "alpha", "m", "/d", "2026-03-02T00:00:00", vec![], ""),
            make_note("c", "beta", "m", "/d", "2026-03-01T00:00:00", vec![], ""),
        ];
        let groups = group_by_repo(&notes, None);
        let alpha = groups.iter().find(|g| g.repo == "alpha").unwrap();
        let beta = groups.iter().find(|g| g.repo == "beta").unwrap();
        assert_eq!(alpha.note_count, 2);
        assert_eq!(beta.note_count, 1);
    }
}
