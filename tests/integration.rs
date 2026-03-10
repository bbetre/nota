//! Integration tests — run the compiled `nota` binary against a temp notes dir.
//!
//! Every test sets `NOTA_NOTES_DIR` to a fresh `tempdir()` so tests are fully
//! isolated from each other and from the user's real `~/.notes/`.

use std::path::Path;
use std::process::Command;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Path to the debug binary built by Cargo.
fn bin() -> std::path::PathBuf {
    // CARGO_BIN_EXE_nota is set by Cargo when running integration tests.
    // Falls back to a relative path for environments where the env var is absent.
    let exe = env!("CARGO_BIN_EXE_nota");
    std::path::PathBuf::from(exe)
}

/// Run `nota <args>` with a dedicated temp notes directory.
/// Returns (stdout, stderr, exit_status).
fn run(dir: &Path, args: &[&str]) -> (String, String, std::process::ExitStatus) {
    let out = Command::new(bin())
        .args(args)
        .env("NOTA_NOTES_DIR", dir)
        // Disable git discovery so tests are repo-agnostic
        .env("GIT_DIR", "")
        .env("GIT_CEILING_DIRECTORIES", "/")
        .output()
        .expect("failed to run nota binary");

    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status,
    )
}

/// Create a temp dir and return it (keeps the dir alive for the test scope).
fn tmpdir() -> tempfile::TempDir {
    tempfile::tempdir().expect("failed to create temp dir")
}

// ── nota add ──────────────────────────────────────────────────────────────────

#[test]
fn add_creates_note_file() {
    let dir = tmpdir();
    let (stdout, stderr, status) = run(dir.path(), &["add", "hello integration test"]);
    assert!(status.success(), "stderr: {}", stderr);
    assert!(stdout.contains("Saved note"), "stdout: {}", stdout);

    // There should be exactly one .md file in the temp dir
    let files: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("md"))
        .collect();
    assert_eq!(files.len(), 1, "expected 1 note file");
}

#[test]
fn add_empty_body_exits_nonzero() {
    let dir = tmpdir();
    // Pass an empty string as inline text
    let (_, stderr, status) = run(dir.path(), &["add", ""]);
    assert!(!status.success());
    assert!(stderr.contains("empty"), "stderr: {}", stderr);
}

#[test]
fn add_note_frontmatter_contains_correct_fields() {
    let dir = tmpdir();
    run(dir.path(), &["add", "frontmatter check"]);

    let file = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .find(|e| e.path().extension().and_then(|s| s.to_str()) == Some("md"))
        .expect("no note file found");

    let content = std::fs::read_to_string(file.path()).unwrap();
    assert!(content.contains("id:"), "missing id field");
    assert!(content.contains("timestamp:"), "missing timestamp field");
    assert!(content.contains("directory:"), "missing directory field");
    assert!(content.contains("git_repo:"), "missing git_repo field");
    assert!(content.contains("git_branch:"), "missing git_branch field");
    assert!(content.contains("frontmatter check"), "body not written");
}

// ── nota list ─────────────────────────────────────────────────────────────────

#[test]
fn list_shows_added_note() {
    let dir = tmpdir();
    run(dir.path(), &["add", "my list test note"]);
    let (stdout, _, status) = run(dir.path(), &["list"]);
    assert!(status.success());
    assert!(stdout.contains("my list test note"), "stdout: {}", stdout);
}

#[test]
fn list_empty_dir_shows_nothing() {
    let dir = tmpdir();
    let (stdout, _, status) = run(dir.path(), &["list"]);
    assert!(status.success());
    assert!(
        stdout.trim().is_empty(),
        "expected no output, got: {}",
        stdout
    );
}

#[test]
fn list_tag_filter() {
    let dir = tmpdir();
    // Add two notes
    let (out, _, _) = run(dir.path(), &["add", "tagged note"]);
    // Extract ID from "Saved note <id>."
    let id = out
        .trim()
        .split_whitespace()
        .nth(2)
        .unwrap()
        .trim_end_matches('.');
    // Tag the first note
    run(dir.path(), &["tag", "add", id, "mytag"]);
    run(dir.path(), &["add", "untagged note"]);

    let (stdout, _, status) = run(dir.path(), &["list", "--tag", "mytag"]);
    assert!(status.success());
    assert!(stdout.contains("tagged note"), "stdout: {}", stdout);
    assert!(
        !stdout.contains("untagged note"),
        "should not show untagged: {}",
        stdout
    );
}

// ── nota show ─────────────────────────────────────────────────────────────────

#[test]
fn show_prints_body() {
    let dir = tmpdir();
    let (out, _, _) = run(dir.path(), &["add", "show me this body"]);
    let id = out
        .trim()
        .split_whitespace()
        .nth(2)
        .unwrap()
        .trim_end_matches('.');
    let (stdout, _, status) = run(dir.path(), &["show", id]);
    assert!(status.success());
    assert!(stdout.contains("show me this body"), "stdout: {}", stdout);
}

#[test]
fn show_nonexistent_id_exits_nonzero() {
    let dir = tmpdir();
    let (_, stderr, status) = run(dir.path(), &["show", "deadbeef"]);
    assert!(!status.success());
    assert!(stderr.contains("not found"), "stderr: {}", stderr);
}

// ── nota search ───────────────────────────────────────────────────────────────

#[test]
fn search_finds_matching_note() {
    let dir = tmpdir();
    run(dir.path(), &["add", "the quick brown fox"]);
    run(dir.path(), &["add", "something else entirely"]);
    let (stdout, _, status) = run(dir.path(), &["search", "quick"]);
    assert!(status.success());
    assert!(stdout.contains("quick brown fox"), "stdout: {}", stdout);
    assert!(!stdout.contains("something else"), "stdout: {}", stdout);
}

#[test]
fn search_case_insensitive() {
    let dir = tmpdir();
    run(dir.path(), &["add", "Check the Auth middleware"]);
    let (stdout, _, status) = run(dir.path(), &["search", "AUTH"]);
    assert!(status.success());
    assert!(stdout.contains("Auth"), "stdout: {}", stdout);
}

// ── nota tag ──────────────────────────────────────────────────────────────────

#[test]
fn tag_add_then_rm() {
    let dir = tmpdir();
    let (out, _, _) = run(dir.path(), &["add", "tag round-trip test"]);
    let id = out
        .trim()
        .split_whitespace()
        .nth(2)
        .unwrap()
        .trim_end_matches('.');

    // Add tags
    let (stdout, _, status) = run(dir.path(), &["tag", "add", id, "rust", "cli"]);
    assert!(status.success(), "tag add failed");
    assert!(stdout.contains("rust"), "stdout: {}", stdout);

    // Verify they appear in list
    let (stdout, _, _) = run(dir.path(), &["list"]);
    assert!(
        stdout.contains("#rust") || stdout.contains("rust"),
        "tags not shown: {}",
        stdout
    );

    // Remove one tag
    let (stdout, _, status) = run(dir.path(), &["tag", "rm", id, "rust"]);
    assert!(status.success(), "tag rm failed");
    assert!(stdout.contains("Removed 1"), "stdout: {}", stdout);

    // Verify rust is gone but cli remains
    let file = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .find(|e| e.path().extension().and_then(|s| s.to_str()) == Some("md"))
        .unwrap();
    let content = std::fs::read_to_string(file.path()).unwrap();
    assert!(!content.contains("rust"), "rust tag should be removed");
    assert!(content.contains("cli"), "cli tag should remain");
}

#[test]
fn tag_add_deduplicates() {
    let dir = tmpdir();
    let (out, _, _) = run(dir.path(), &["add", "dedup test"]);
    let id = out
        .trim()
        .split_whitespace()
        .nth(2)
        .unwrap()
        .trim_end_matches('.');

    run(dir.path(), &["tag", "add", id, "rust"]);
    let (stdout, _, _) = run(dir.path(), &["tag", "add", id, "rust"]);
    assert!(
        stdout.contains("No new tags"),
        "should deduplicate: {}",
        stdout
    );
}

#[test]
fn tag_nonexistent_id_exits_nonzero() {
    let dir = tmpdir();
    let (_, stderr, status) = run(dir.path(), &["tag", "add", "deadbeef", "mytag"]);
    assert!(!status.success());
    assert!(stderr.contains("not found"), "stderr: {}", stderr);
}

// ── nota stats ────────────────────────────────────────────────────────────────

#[test]
fn stats_shows_totals() {
    let dir = tmpdir();
    run(dir.path(), &["add", "stats note one"]);
    run(dir.path(), &["add", "stats note two"]);
    let (stdout, _, status) = run(dir.path(), &["stats"]);
    assert!(status.success());
    assert!(stdout.contains("Total"), "stdout: {}", stdout);
    assert!(stdout.contains('2'), "stdout: {}", stdout);
}

#[test]
fn stats_empty_dir() {
    let dir = tmpdir();
    let (stdout, _, status) = run(dir.path(), &["stats"]);
    assert!(status.success());
    assert!(stdout.contains('0'), "stdout: {}", stdout);
}

// ── nota completions ──────────────────────────────────────────────────────────

#[test]
fn completions_zsh_outputs_script() {
    let dir = tmpdir();
    let (stdout, _, status) = run(dir.path(), &["completions", "zsh"]);
    assert!(status.success());
    assert!(!stdout.is_empty(), "completions output should not be empty");
    // zsh completions always start with #compdef
    assert!(stdout.contains("nota"), "should mention the binary name");
}

#[test]
fn completions_bash_outputs_script() {
    let dir = tmpdir();
    let (stdout, _, status) = run(dir.path(), &["completions", "bash"]);
    assert!(status.success());
    assert!(!stdout.is_empty());
}

#[test]
fn completions_fish_outputs_script() {
    let dir = tmpdir();
    let (stdout, _, status) = run(dir.path(), &["completions", "fish"]);
    assert!(status.success());
    assert!(!stdout.is_empty());
}

// ── nota log ──────────────────────────────────────────────────────────────────

#[test]
fn log_shows_repo_groups() {
    let dir = tmpdir();
    run(dir.path(), &["add", "log test note"]);
    let (stdout, _, status) = run(dir.path(), &["log"]);
    assert!(status.success());
    // Should have at least one line (even if repo is "none")
    assert!(!stdout.trim().is_empty(), "log output should not be empty");
}

// ── nota delete ───────────────────────────────────────────────────────────────

#[test]
fn delete_nonexistent_exits_nonzero() {
    let dir = tmpdir();
    let (_, stderr, status) = run(dir.path(), &["delete", "deadbeef"]);
    assert!(!status.success());
    assert!(stderr.contains("not found"), "stderr: {}", stderr);
}
