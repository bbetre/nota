use crate::note::Note;
use crate::search::{NoteStats, RepoGroup};
use colored::Colorize;

// ── Config ────────────────────────────────────────────────────────────────────

pub struct DisplayConfig {
    pub body_width: usize,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self { body_width: 80 }
    }
}

// ── Formatting helpers ────────────────────────────────────────────────────────

pub fn format_timestamp(ts: &str) -> String {
    match crate::note::parse_timestamp(ts) {
        Some(dt) => dt.format("%Y-%m-%d %H:%M").to_string(),
        None => ts.to_string(),
    }
}

pub fn truncate_body(body: &str, width: usize) -> String {
    // Use only the first line for table display
    let first_line = body.lines().next().unwrap_or("").trim();
    let mut chars = first_line.chars();
    let truncated: String = chars.by_ref().take(width).collect();
    if chars.next().is_some() {
        format!("{}…", truncated)
    } else {
        truncated.to_string()
    }
}

fn format_tags(tags: &[String]) -> String {
    if tags.is_empty() {
        return String::new();
    }
    tags.iter()
        .map(|t| format!("#{}", t).cyan().dimmed().to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

fn highlight_matches(text: &str, query: &str) -> String {
    let lower_text = text.to_lowercase();
    let lower_query = query.to_lowercase();
    let mut result = String::with_capacity(text.len());
    let mut start = 0usize;

    while start < lower_text.len() {
        match lower_text[start..].find(&lower_query) {
            None => {
                result.push_str(&text[start..]);
                break;
            }
            Some(rel_pos) => {
                let abs_pos = start + rel_pos;
                let match_end = abs_pos + query.len();

                // Text before the match — uncolored
                result.push_str(&text[start..abs_pos]);
                // Matched text — yellow bold, preserving original case
                result.push_str(&text[abs_pos..match_end].yellow().bold().to_string());
                start = match_end;
            }
        }
    }
    result
}

// ── Printers ──────────────────────────────────────────────────────────────────

pub fn print_notes_table(notes: &[Note], config: &DisplayConfig) {
    for note in notes {
        let ts = format_timestamp(&note.frontmatter.timestamp)
            .dimmed()
            .to_string();
        let ctx = format!(
            "{}/{}",
            note.frontmatter.git_repo, note.frontmatter.git_branch
        )
        .blue()
        .to_string();
        let body = truncate_body(&note.body, config.body_width)
            .white()
            .to_string();
        let id = format!("[{}]", note.frontmatter.id).dimmed().to_string();
        let tags = format_tags(&note.frontmatter.tags);

        if tags.is_empty() {
            println!("{}  {}  {}  {}", ts, ctx, body, id);
        } else {
            println!("{}  {}  {}  {}  {}", ts, ctx, body, tags, id);
        }
    }
}

pub fn print_notes_with_highlight(notes: &[Note], query: &str, config: &DisplayConfig) {
    for note in notes {
        let ts = format_timestamp(&note.frontmatter.timestamp)
            .dimmed()
            .to_string();
        let ctx = format!(
            "{}/{}",
            note.frontmatter.git_repo, note.frontmatter.git_branch
        )
        .blue()
        .to_string();
        let truncated = truncate_body(&note.body, config.body_width);
        let body = highlight_matches(&truncated, query);
        let id = format!("[{}]", note.frontmatter.id).dimmed().to_string();
        let tags = format_tags(&note.frontmatter.tags);

        if tags.is_empty() {
            println!("{}  {}  {}  {}", ts, ctx, body, id);
        } else {
            println!("{}  {}  {}  {}  {}", ts, ctx, body, tags, id);
        }
    }
}

pub fn print_log_table(groups: &[RepoGroup], _config: &DisplayConfig) {
    for group in groups {
        let repo = group.repo.blue().to_string();
        let count = if group.note_count == 1 {
            "1 note ".to_string()
        } else {
            format!("{} notes", group.note_count)
        };
        let last = format!("last: {}", group.last_activity)
            .dimmed()
            .to_string();
        println!("{:<30}  {:<10}  {}", repo, count, last);
    }
}

pub fn print_changed_files(files: &[String]) {
    if files.is_empty() {
        return;
    }
    println!("\n{}", "── staged files ─────────────────────".dimmed());
    for f in files {
        println!("  {}", f.dimmed());
    }
}

pub fn print_stats(stats: &NoteStats) {
    println!("{}", "── nota stats ──────────────────────".dimmed());
    println!(
        "  {:<18} {}",
        "Total notes".dimmed(),
        stats.total.to_string().white().bold()
    );
    println!(
        "  {:<18} {}",
        "Today".dimmed(),
        stats.today.to_string().white()
    );
    println!(
        "  {:<18} {}",
        "This week".dimmed(),
        stats.this_week.to_string().white()
    );
    println!(
        "  {:<18} {}",
        "This month".dimmed(),
        stats.this_month.to_string().white()
    );
    println!("{}", "────────────────────────────────────".dimmed());
    match &stats.most_active_repo {
        Some(repo) => println!(
            "  {:<18} {} {} {}",
            "Most active repo".dimmed(),
            repo.blue().bold(),
            "·".dimmed(),
            format!("{} notes", stats.most_active_repo_count).dimmed()
        ),
        None => println!("  {:<18} {}", "Most active repo".dimmed(), "—".dimmed()),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_body_short_string() {
        let s = truncate_body("hello", 80);
        assert_eq!(s, "hello");
    }

    #[test]
    fn truncate_body_at_boundary() {
        let s = truncate_body("abcde", 5);
        assert_eq!(s, "abcde");
    }

    #[test]
    fn truncate_body_over_limit() {
        let s = truncate_body("abcdef", 5);
        assert_eq!(s, "abcde…");
    }

    #[test]
    fn truncate_body_uses_first_line_only() {
        let s = truncate_body("line one\nline two", 80);
        assert_eq!(s, "line one");
    }

    #[test]
    fn format_timestamp_valid() {
        let s = format_timestamp("2026-03-05T14:32:00");
        assert_eq!(s, "2026-03-05 14:32");
    }

    #[test]
    fn format_timestamp_invalid_passthrough() {
        let s = format_timestamp("not-a-date");
        assert_eq!(s, "not-a-date");
    }

    #[test]
    fn highlight_matches_wraps_term() {
        // With color disabled we can't test ANSI codes, but we can verify
        // the function returns something containing the matched text.
        colored::control::set_override(false);
        let result = highlight_matches("Check the Auth middleware", "auth");
        assert!(result.contains("Auth"));
    }

    #[test]
    fn highlight_matches_no_match() {
        colored::control::set_override(false);
        let result = highlight_matches("nothing here", "xyz");
        assert_eq!(result, "nothing here");
    }
}
