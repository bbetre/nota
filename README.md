# nota — Contextual Note Taker

> "The note should answer: what was I doing, where was I, and what matters right now?"

A fast, zero-friction CLI note-taking tool written in Rust. Every note is automatically tagged with your current git repo, branch, working directory, and any staged files — so you can find notes by *where you were*, not just *what you wrote*.

Notes are plain Markdown files stored in `~/.notes/`. No database, no daemon, no sync service.

---

## Features

- **Context capture** — Records git repo, branch, directory, commit, and timestamp automatically on every `nota add`
- **Staged file tracking** — Captures which files you had staged in git at the time of writing
- **Commit linking** — Find notes by the commit they were created in; see commit info in `nota show`
- **Tags** — Add freeform tags to any note; filter by tag in `list` and `search`
- **Search** — Full-text search with `--fuzzy` flag for typo-tolerant, flexible matching
- **Interactive TUI** — `nota tui` for keyboard-driven browsing, searching, and tagging
- **Stats** — See note counts by day/week/month and your most active repo
- **Shell completions** — Bash, Zsh, and Fish supported via `nota completions`
- **Plain text** — Every note is a readable Markdown file; open, edit, or grep them directly
- **Zero friction** — `nota add "your note"` is all it takes

---
graph TD
    A[AI Coding Agents] --> B[InsForge Semantic Layer]
    B --> C[Authentication]
    B --> D[Database]
    B --> E[Storage]
    B --> F[Edge Functions]
    B --> G[Model Gateway]
    B --> H[Policy]
---

## Install

```bash
git clone https://github.com/bbetre/nota.git
cd nota
cargo install --path .
```

Requires Rust 1.75+. Binary is placed at `~/.cargo/bin/nota`.

---

## Quick Start

```bash
# Save a note (context captured automatically)
nota add "Fix the N+1 query in users endpoint before deploy"

# List recent notes
nota list

# Show notes from your current repo/directory
nota context

# Search across all notes
nota search "N+1"

# Fuzzy search for typo tolerance
nota search "databse" --fuzzy     # Finds "database" despite typo

# View notes from a specific commit
nota commits abc1234    # Find all notes from this commit

# Add tags to a note
nota tag add a3f9c1bd performance database

# Filter by tag
nota list --tag performance

# View your note-taking activity
nota stats
```

---

## Commands

### `nota add [TEXT]`

Save a new note. Context (repo, branch, directory, staged files) is captured automatically.

```bash
nota add "Remember to update the migration before merging"
# → Saved note a3f9c1bd.

# Pipe text in from stdin
echo "Deploy blocked — waiting on infra team" | nota add

# Interactive multi-line input (Ctrl+D to save)
nota add
```

---

### `nota list`

List notes, most recent first.

```
nota list [--limit N] [--here] [--repo NAME] [--branch NAME] [--tag TAG]
```

| Flag | Description |
|---|---|
| `--limit N` | Maximum notes to show (default: 20) |
| `--here` | Filter to current repo **and** current directory |
| `--repo NAME` | Filter by repository name |
| `--branch NAME` | Filter by branch name |
| `--tag TAG` | Filter by tag |

```bash
nota list
nota list --limit 50
nota list --here
nota list --repo nota --branch main
nota list --tag bug
```

---

### `nota show ID`

Print a note's full body to stdout. Also displays staged files and commit info if available.

```bash
nota show a3f9c1bd
```

```
Fix the N+1 query in users endpoint before deploy

── commit a7f3c2d1
── staged ────────────────────────────
  src/db/users.rs
  src/handlers/users.rs
```

Output is pipe-friendly (no ANSI color when piped).

---

### `nota commits HASH`

List all notes created during a specific commit.

```bash
nota commits a7f3c2d1      # Full or short commit hash
nota commits a7f3c2d       # Short hash (7+ chars)
```

Useful for finding all the notes you wrote while working on a particular commit, or for linking development context with commit history.

---

### `nota search QUERY`

Full-text search across all note bodies, with matches highlighted.

```
nota search QUERY [--here] [--tag TAG] [--fuzzy]
```

| Flag | Description |
|---|---|
| `--here` | Scope search to current repo and directory |
| `--tag TAG` | Filter results to notes with this tag |
| `--fuzzy` | Use fuzzy matching for typo-tolerant search |

```bash
nota search "migration"
nota search "deploy" --here
nota search "timeout" --tag backend
nota search "detabase" --fuzzy    # Finds "database" despite typo
nota search "DB" --fuzzy --tag backend  # Fuzzy + filters combined
```

**Fuzzy vs. exact search:** By default, `nota search` uses case-insensitive substring matching. With `--fuzzy`, it uses flexible matching that tolerates typos, skipped characters, and word order changes. Results are ranked by match quality (best matches first). Useful when you're not sure of exact spelling or phrasing.

---

### `nota tui`

Launch an interactive terminal user interface for browsing and managing notes with keyboard control.

```bash
nota tui
```

**Keybindings in TUI mode:**

| Key | Action |
|---|---|
| `j` / `↓` | Move down one note |
| `k` / `↑` | Move up one note |
| `Home` / `End` | Jump to first/last note |
| `/` | Enter search mode (filter notes by query) |
| `Enter` | Exit search mode (confirm filter) |
| `Esc` | Cancel search/exit tag input/exit TUI |
| `t` | Enter tag input mode (add tags) |
| `Space` | Accept tag when in tag input mode |
| `q` | Quit TUI |

The TUI displays notes in a two-pane layout:
- **Left pane** — List of notes, most recent first
- **Right pane** — Full preview of selected note, including metadata and body

Search mode filters the note list in real-time. Tag input mode allows you to add space-separated tags to the selected note. The help text at the bottom updates to show available actions in your current mode.

```bash
nota tui
# → Opens interactive terminal with all your notes
# → Press 'j'/'k' to navigate, '/' to search, 't' to tag, 'q' to quit
```

---

### `nota context`

Show notes from your current repo and working directory. Equivalent to `nota list --here`.

```
nota context [--repo NAME] [--branch NAME]
```

When `--repo` or `--branch` are provided, filters by those values without the directory constraint.

```bash
nota context                    # notes from this repo + this dir
nota context --repo myapp       # all notes from myapp, any branch
nota context --branch feat/auth # all notes on feat/auth branch
```

---

### `nota log`

Activity log grouped by repository, showing note counts and recency.

```
nota log [--days N]
```

```bash
nota log            # all time
nota log --days 7   # last 7 days
```

---

### `nota tag add ID TAG...`

Add one or more tags to a note. Tags are stored lowercase and deduplicated.

```bash
nota tag add a3f9c1bd performance
nota tag add a3f9c1bd bug urgent backend
```

---

### `nota tag rm ID TAG...`

Remove one or more tags from a note.

```bash
nota tag rm a3f9c1bd urgent
```

---

### `nota stats`

Show an overview of your note-taking activity.

```bash
nota stats
```

```
Total notes:       142
Today:               3
This week:          17
This month:         41
Most active repo:  myapp (58 notes)
```

---

### `nota delete ID`

Delete a note by ID. Prompts for confirmation.

```bash
nota delete a3f9c1bd
# → Delete note a3f9c1bd? [y/N]
```

---

### `nota edit ID`

Open a note in `$EDITOR` (falls back to `$VISUAL`).

```bash
nota edit a3f9c1bd
```

---

### `nota completions SHELL`

Print a shell completion script to stdout.

```bash
# Zsh
nota completions zsh >> ~/.zshrc

# Bash
nota completions bash >> ~/.bashrc

# Fish
nota completions fish > ~/.config/fish/completions/nota.fish
```

Supported shells: `bash`, `zsh`, `fish`.

---

## Note File Format

Each note is a plain Markdown file stored at `~/.notes/<id>.md`.

```markdown
---
id: a3f9c1bd
timestamp: '2026-03-10T14:32:00'
directory: /Users/you/projects/myapp
git_repo: myapp
git_branch: feat/users
tags:
- performance
- database
changed_files:
- src/db/users.rs
- src/handlers/users.rs
---

Fix the N+1 query in the users endpoint before deploy.
```

- `tags` and `changed_files` are omitted from the file when empty — old notes remain valid.
- You can edit notes directly; the YAML frontmatter is re-parsed on each read.
- Notes directory can be overridden with the `NOTA_NOTES_DIR` environment variable.

---

## Architecture

```
src/
├── main.rs       CLI entry point — command definitions (clap) and handlers
├── note.rs       NoteFrontmatter struct, file I/O, ID generation, YAML parsing
├── context.rs    capture_context() — git repo/branch/staged files via git2
├── search.rs     FilterOptions, load_all_notes(), search_notes(), compute_stats()
└── display.rs    Terminal formatting — tables, highlights, tags, staged files
```

| Module | Responsibility |
|---|---|
| `main.rs` | Parses CLI args, dispatches to command handlers |
| `note.rs` | Defines the `Note` and `NoteFrontmatter` types; reads/writes `.md` files |
| `context.rs` | Captures git context (repo name, branch, staged files) using `git2` |
| `search.rs` | Filtering, sorting, full-text search, grouping, and stats computation |
| `display.rs` | All terminal output — colored tables, search highlighting, stats formatting |

---

## Running Tests

```bash
cargo test          # 55 tests: 32 unit + 23 integration
cargo clippy -- -D warnings
cargo fmt --check
```

Integration tests use `NOTA_NOTES_DIR` to write to a temp directory and never touch `~/.notes/`.

---

## Known Limitations

- **Search scalability** — Notes are loaded from disk on every query. Works well up to ~10,000 notes; beyond that, SQLite FTS would be the natural next step.
- **No sync** — Notes live only on the local machine. Sync via git (`git init ~/.notes`) is a reasonable workaround.
- **Single binary** — There is no daemon or background process; context is captured synchronously at write time.
- **ID collisions** — IDs are 8-char hex prefixes of UUID v4. Collision probability is negligible in practice (~1 in 4 billion per note).
