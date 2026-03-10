# nota — Contextual Note Taker

> "The note should answer: what was I doing, where was I, and what matters right now?"

A fast, zero-friction CLI note-taking tool written in Rust. Every note is automatically tagged with your current git repo, branch, working directory, and any staged files — so you can find notes by *where you were*, not just *what you wrote*.

Notes are plain Markdown files stored in `~/.notes/`. No database, no daemon, no sync service.

---

## Features

- **Context capture** — Records git repo, branch, directory, and timestamp automatically on every `nota add`
- **Staged file tracking** — Captures which files you had staged in git at the time of writing
- **Tags** — Add freeform tags to any note; filter by tag in `list` and `search`
- **Stats** — See note counts by day/week/month and your most active repo
- **Shell completions** — Bash, Zsh, and Fish supported via `nota completions`
- **Plain text** — Every note is a readable Markdown file; open, edit, or grep them directly
- **Zero friction** — `nota add "your note"` is all it takes

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

Print a note's full body to stdout. Also displays staged files if any were captured.

```bash
nota show a3f9c1bd
```

```
Fix the N+1 query in users endpoint before deploy

── staged files ──────────────────────────────────
  src/db/users.rs
  src/handlers/users.rs
```

Output is pipe-friendly (no ANSI color when piped).

---

### `nota search QUERY`

Full-text search across all note bodies, with matches highlighted.

```
nota search QUERY [--here] [--tag TAG]
```

| Flag | Description |
|---|---|
| `--here` | Scope search to current repo and directory |
| `--tag TAG` | Filter results to notes with this tag |

```bash
nota search "migration"
nota search "deploy" --here
nota search "timeout" --tag backend
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
