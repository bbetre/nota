use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};

// ── Structs ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteFrontmatter {
    pub id: String,
    pub timestamp: String,
    pub directory: String,
    pub git_repo: String,
    pub git_branch: String,
    /// Current commit hash (40-char SHA-1) at the time the note was saved.
    /// Absent in old notes — defaults to "none".
    #[serde(default = "default_none")]
    pub commit_hash: String,
    /// Optional freeform tags. Absent in old notes — defaults to empty vec.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Staged git files at the time the note was saved (relative to repo root).
    /// Absent in old notes — defaults to empty vec.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changed_files: Vec<String>,
    /// Unstaged modified files at the time the note was saved (relative to repo root).
    /// Absent in old notes — defaults to empty vec.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unstaged_files: Vec<String>,
    /// Untracked files (new, never git add'd) at the time the note was saved.
    /// Absent in old notes — defaults to empty vec.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub untracked_files: Vec<String>,
}

fn default_none() -> String {
    "none".to_string()
}

#[derive(Debug, Clone)]
pub struct Note {
    pub frontmatter: NoteFrontmatter,
    pub body: String,
    pub file_path: PathBuf,
}

// ── Error type ────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum NoteParseError {
    MissingFrontmatter,
    InvalidYaml(serde_yaml::Error),
}

impl fmt::Display for NoteParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NoteParseError::MissingFrontmatter => write!(f, "missing or malformed frontmatter"),
            NoteParseError::InvalidYaml(e) => write!(f, "invalid YAML: {}", e),
        }
    }
}

impl std::error::Error for NoteParseError {}

impl From<serde_yaml::Error> for NoteParseError {
    fn from(e: serde_yaml::Error) -> Self {
        NoteParseError::InvalidYaml(e)
    }
}

// ── Paths ─────────────────────────────────────────────────────────────────────

/// Returns the notes directory. Respects `NOTA_NOTES_DIR` env var override
/// (used by tests to avoid touching `~/.notes`).
pub fn notes_dir() -> PathBuf {
    if let Ok(override_dir) = std::env::var("NOTA_NOTES_DIR") {
        return PathBuf::from(override_dir);
    }
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".notes")
}

pub fn ensure_notes_dir() -> Result<PathBuf, std::io::Error> {
    let dir = notes_dir();
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

// ── ID generation ─────────────────────────────────────────────────────────────

pub fn generate_id() -> String {
    // UUID v4 string: "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx"
    // First 8 chars are always lowercase hex digits before the first hyphen.
    uuid::Uuid::new_v4().to_string()[..8].to_string()
}

// ── Timestamp parsing ─────────────────────────────────────────────────────────

pub fn parse_timestamp(ts: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(ts, "%Y-%m-%dT%H:%M:%S").ok()
}

// ── File parsing ──────────────────────────────────────────────────────────────

pub fn parse_note_file(content: &str, path: &Path) -> Result<Note, NoteParseError> {
    let mut lines = content.lines();

    // First line must be "---"
    if lines.next().map(str::trim) != Some("---") {
        return Err(NoteParseError::MissingFrontmatter);
    }

    // Collect YAML lines until closing "---"
    let mut yaml_lines: Vec<&str> = Vec::new();
    let mut found_close = false;
    for line in lines.by_ref() {
        if line.trim() == "---" {
            found_close = true;
            break;
        }
        yaml_lines.push(line);
    }

    if !found_close {
        return Err(NoteParseError::MissingFrontmatter);
    }

    let yaml_block = yaml_lines.join("\n");
    let frontmatter: NoteFrontmatter = serde_yaml::from_str(&yaml_block)?;

    // Everything remaining after the closing "---" is the body
    let body: String = lines.collect::<Vec<_>>().join("\n");
    let body = body.trim_start_matches('\n').trim_end().to_string();

    Ok(Note {
        frontmatter,
        body,
        file_path: path.to_path_buf(),
    })
}

// ── File writing ──────────────────────────────────────────────────────────────

pub fn write_note(note: &Note) -> Result<(), std::io::Error> {
    let yaml = serde_yaml::to_string(&note.frontmatter)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    // serde_yaml 0.9 emits a leading "---\n" automatically; strip it so we control the format.
    let yaml_body = yaml.trim_start_matches("---\n");
    let content = format!("---\n{}---\n\n{}\n", yaml_body, note.body.trim_end());
    let path = notes_dir().join(format!("{}.md", note.frontmatter.id));
    std::fs::write(path, content)
}

// ── Lookup by ID ──────────────────────────────────────────────────────────────

pub fn load_note_by_id(id: &str) -> Result<Note, Box<dyn std::error::Error>> {
    let path = notes_dir().join(format!("{}.md", id));
    if !path.exists() {
        return Err(format!("note '{}' not found.", id).into());
    }
    let content = std::fs::read_to_string(&path)?;
    parse_note_file(&content, &path).map_err(|e| e.into())
}

pub fn delete_note_by_id(id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = notes_dir().join(format!("{}.md", id));
    if !path.exists() {
        return Err(format!("note '{}' not found.", id).into());
    }
    std::fs::remove_file(path).map_err(|e| e.into())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn dummy_path() -> PathBuf {
        PathBuf::from("/tmp/test_note.md")
    }

    fn valid_note_content(tags: &str) -> String {
        format!(
            "---\nid: abcd1234\ntimestamp: 2026-03-05T14:32:00\ndirectory: /home/user/proj\ngit_repo: proj\ngit_branch: main\n{tags}---\n\nThis is the body.\n",
            tags = tags
        )
    }

    #[test]
    fn parse_valid_note_no_tags() {
        let content = valid_note_content("");
        let note = parse_note_file(&content, &dummy_path()).unwrap();
        assert_eq!(note.frontmatter.id, "abcd1234");
        assert_eq!(note.frontmatter.git_repo, "proj");
        assert_eq!(note.frontmatter.git_branch, "main");
        assert_eq!(note.frontmatter.tags, Vec::<String>::new());
        assert_eq!(note.body, "This is the body.");
    }

    #[test]
    fn parse_valid_note_with_tags() {
        let content = valid_note_content("tags:\n- rust\n- cli\n");
        let note = parse_note_file(&content, &dummy_path()).unwrap();
        assert_eq!(note.frontmatter.tags, vec!["rust", "cli"]);
    }

    #[test]
    fn parse_missing_frontmatter() {
        let content = "No frontmatter here\njust plain text\n";
        let result = parse_note_file(content, &dummy_path());
        assert!(matches!(result, Err(NoteParseError::MissingFrontmatter)));
    }

    #[test]
    fn parse_unclosed_frontmatter() {
        let content = "---\nid: abcd1234\ntimestamp: 2026-03-05T14:32:00\n";
        let result = parse_note_file(content, &dummy_path());
        assert!(matches!(result, Err(NoteParseError::MissingFrontmatter)));
    }

    #[test]
    fn parse_invalid_yaml() {
        let content = "---\n: bad: yaml: [\n---\n\nbody\n";
        let result = parse_note_file(content, &dummy_path());
        assert!(matches!(result, Err(NoteParseError::InvalidYaml(_))));
    }

    #[test]
    fn generate_id_is_8_chars_hex() {
        for _ in 0..20 {
            let id = generate_id();
            assert_eq!(id.len(), 8, "ID should be 8 chars");
            assert!(
                id.chars().all(|c| c.is_ascii_hexdigit()),
                "ID should be hex: {}",
                id
            );
        }
    }

    #[test]
    fn parse_timestamp_valid() {
        let ts = parse_timestamp("2026-03-05T14:32:00");
        assert!(ts.is_some());
    }

    #[test]
    fn parse_timestamp_invalid() {
        let ts = parse_timestamp("not-a-date");
        assert!(ts.is_none());
    }

    #[test]
    fn write_and_parse_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        // Override notes dir for this test
        std::env::set_var("NOTA_NOTES_DIR", dir.path());

        let note = Note {
            frontmatter: NoteFrontmatter {
                id: "deadbeef".to_string(),
                timestamp: "2026-03-05T10:00:00".to_string(),
                directory: "/tmp".to_string(),
                git_repo: "myrepo".to_string(),
                git_branch: "main".to_string(),
                tags: vec!["rust".to_string(), "test".to_string()],
                changed_files: vec!["src/main.rs".to_string(), "src/lib.rs".to_string()],
                unstaged_files: vec!["README.md".to_string()],
                untracked_files: vec!["new_file.rs".to_string()],
            },
            body: "Round-trip test body.".to_string(),
            file_path: dir.path().join("deadbeef.md"),
        };

        write_note(&note).unwrap();

        let path = dir.path().join("deadbeef.md");
        let content = std::fs::read_to_string(&path).unwrap();
        let parsed = parse_note_file(&content, &path).unwrap();

        assert_eq!(parsed.frontmatter.id, "deadbeef");
        assert_eq!(parsed.frontmatter.tags, vec!["rust", "test"]);
        assert_eq!(
            parsed.frontmatter.changed_files,
            vec!["src/main.rs", "src/lib.rs"]
        );
        assert_eq!(parsed.frontmatter.unstaged_files, vec!["README.md"]);
        assert_eq!(parsed.frontmatter.untracked_files, vec!["new_file.rs"]);
        assert_eq!(parsed.body, "Round-trip test body.");

        std::env::remove_var("NOTA_NOTES_DIR");
    }
}
