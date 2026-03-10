# Product Requirements Document

# nota — Contextual Note Taker

| Field       | Value                       |
| ----------- | --------------------------- |
| Binary name | `nota`                      |
| Version     | 1.1                         |
| Status      | Draft                       |
| Language    | Rust                        |
| Storage     | Plain text / Markdown files |
| Date        | March 2026                  |
| Platform    | macOS, Linux, Windows       |

### Revision History

| Version | Changes                                                                                                                                                                                                                                                                              |
| ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1.0     | Initial draft                                                                                                                                                                                                                                                                        |
| 1.1     | Resolved all open questions; added §6.6 (`nota show`); added §7 (Error Handling); clarified `git_repo` derivation rule; settled ID format; resolved `nota list` scope; rationalized `nota context` vs `nota list` flags; added search scalability note; added milestone dependencies |

---

## 1. Overview

> "A note-taking CLI that automatically tags each note with where and when you wrote it — so you can find it later by context, not just content."

Developers constantly switch between projects, branches, and directories. Traditional note apps are context-blind — they don't know you were in the middle of a `feature/auth` branch when you had an idea. Contextual Note Taker (`nota`) solves this by silently capturing the surrounding environment every time a note is created, and letting you query notes by that context later.

---

## 2. Problem Statement

Existing tools fall into two inadequate camps:

- **Generic note apps** (Obsidian, Notion, Apple Notes) — rich features, but completely disconnected from the developer's terminal workflow.
- **Shell history / scratch files** — fast, but unstructured, unsearchable, and not persisted meaningfully.

Neither solution answers the key question a developer actually asks: _"What was I thinking last Tuesday when I was working on the payments service?"_

---

## 3. Goals

| Goal                   | Description                                                                   |
| ---------------------- | ----------------------------------------------------------------------------- |
| Context capture        | Automatically record git repo, branch, directory, and timestamp on every note |
| Fast retrieval         | Find notes by project, branch, or time — not just keyword                     |
| Zero friction          | A note should be saveable in a single short command                           |
| Plain-text portability | Notes are readable Markdown files, not locked in a database                   |
| Showcase-worthy code   | Clean Rust architecture that demonstrates real-world CLI patterns             |

---

## 4. Non-Goals

- No sync or cloud storage (v1)
- No GUI or TUI — pure CLI only
- No collaborative or multi-user features
- No encryption or access control
- Not a replacement for full project management tools
- No subdirectory-based project identity within monorepos (out of scope for v1)

---

## 5. Target Users

**Primary user:** a solo developer working across multiple projects who wants a fast way to leave themselves notes that are automatically tied to the code they were working on.

**Secondary audience:** technical interviewers and portfolio reviewers evaluating the author's Rust and CLI design skills.

---

## 6. Feature Requirements

### 6.1 Note Creation — `P0 Must Have`

```bash
# Inline note
nota add "Tried using Arc instead of Rc here — check perf later"

# Multi-line note via stdin (no arguments triggers stdin mode)
nota add
> Line one
> Line two
> ^D

# Piped input
echo "check the auth middleware" | nota add
```

On execution, the tool silently captures and stores:

- **Timestamp** — ISO 8601 (e.g. `2026-03-05T14:32:00`)
- **Current directory** — absolute path
- **Git repository name** — see derivation rule below
- **Active git branch** — current HEAD branch name (e.g. `feature/auth`); stored as `none` if in detached HEAD state

If the user is not inside a git repo, `git_repo` and `git_branch` are stored as `none`. Each note is saved as a separate `.md` file in `~/.notes/` with a YAML frontmatter header.

#### git_repo Derivation Rule

Repo name is derived using the following priority order:

1. **Remote origin URL** — parse the final path segment, stripping the `.git` suffix.
   - HTTPS: `https://github.com/user/my-app.git` → `my-app`
   - SSH: `git@github.com:user/my-app.git` → `my-app`
2. **No remote origin** — fall back to the name of the root directory of the git repo (i.e. the folder containing `.git/`).
3. **Multiple remotes** — always use `origin`. If `origin` does not exist, use the first remote alphabetically.
4. **Not a git repo** — store as `none`.

Subdirectory paths within a monorepo are intentionally ignored for v1. The full `directory` field captures the working path regardless.

#### ID Format

Each note is assigned an **8-character lowercase hex ID**, derived from the first 8 characters of a UUID v4. Example: `a3f9c1bd`.

Rationale: short enough to type for `nota delete`, long enough to make collisions negligible at personal note volumes. The full UUID is stored internally; only the 8-char prefix is shown to users and used in filenames.

### 6.2 Note File Format — `P0 Must Have`

```markdown
---
id: a3f9c1bd
timestamp: 2026-03-05T14:32:00
directory: /home/user/projects/my-app
git_repo: my-app
git_branch: feature/auth
---

Tried using Arc instead of Rc here — check perf later.
```

Files are named by their 8-char ID: `~/.notes/a3f9c1bd.md`.

### 6.3 List Notes — `P0 Must Have`

`nota list` shows **all notes globally**, sorted by most recent first. This matches the mental model of `git log` — you get everything by default and narrow with flags.

```bash
# Show the 20 most recent notes (default)
nota list

# Show more
nota list --limit 50

# Narrow to current repo + directory
nota list --here

# Narrow to a specific repo or branch
nota list --repo my-app
nota list --branch feature/auth
```

Output is a formatted, color-coded table showing timestamp, repo/branch, and note body (truncated at 80 chars).

### 6.4 Full-Text Search — `P0 Must Have`

```bash
# Search note bodies for a keyword or phrase
nota search "Arc"

# Scope search to current repo
nota search "Arc" --here
```

Performs case-insensitive substring search across all note bodies. Matching terms are highlighted in the output. Results are sorted by recency.

**Scalability note:** In-process `walkdir` + string matching is sufficient for up to ~10,000 notes with negligible latency. Beyond that, a persistent index (e.g. a local SQLite FTS table) would be needed. This is out of scope for v1 but should be acknowledged in the README as a known future limitation.

### 6.5 Context Filter — `P0 Must Have`

`nota context` with no flags shows notes matching **both** the current git repo **and** the current directory. Both conditions must match — it is the most focused view available.

```bash
# Notes from current repo AND current directory
nota context

# Notes from a specific repo (any directory within it)
nota context --repo my-app

# Notes from a specific branch
nota context --branch feature/auth

# Notes from a specific repo + branch
nota context --repo my-app --branch feature/auth
```

**Relationship to `nota list`:** `nota list` is the global view with optional narrowing flags. `nota context` is the scoped view — pre-applying the current repo + directory as defaults. They share the same underlying filter logic. Concretely, `nota context` with no flags is equivalent to `nota list --here`; both apply the same repo + directory filter. The two commands exist as separate UX affordances, not separate implementations.

### 6.6 Show a Note — `P0 Must Have`

```bash
# Print the full content of a note to stdout
nota show a3f9c1bd
```

Prints the complete note body (without frontmatter) to stdout — raw text only, no metadata header. This makes `nota show` safe to pipe to other tools (e.g. `nota show a3f9c1bd | pbcopy`). This is the primary way to read a single note in full, since `nota list` truncates bodies at 80 chars. Exits with a non-zero status code if the ID does not exist.

### 6.7 Activity Log — `P1 Should Have`

```bash
# Show a summary of notes grouped by repo
nota log

# Limit to the last N days
nota log --days 7
```

Groups notes by repository and shows counts and last-activity timestamps — useful as a "what did I work on this week" summary.

### 6.8 Delete a Note — `P1 Should Have`

```bash
nota delete a3f9c1bd
```

Deletes the note file with the given ID. Prompts for confirmation before deletion. Exits with a non-zero status if the ID is not found.

### 6.9 Open in Editor — `P2 Nice to Have`

```bash
# Open a note in $EDITOR for editing
nota edit a3f9c1bd
```

Opens the raw `.md` file in `$EDITOR`. If `$EDITOR` is unset, falls back to `$VISUAL`. If neither is set, prints a clear error:

```
Error: $EDITOR is not set. Set it in your shell profile (e.g. export EDITOR=vim).
```

---

## 7. Error Handling

All errors must produce human-readable messages to stderr and exit with a non-zero status code. `nota` must never panic in release builds.

| Scenario                                          | Behavior                                                                                               |
| ------------------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| `~/.notes/` does not exist                        | Auto-create on first `nota add`. Never error on missing directory.                                     |
| Note file has missing or malformed frontmatter    | Skip silently during list/search; emit a warning to stderr: `warn: skipping malformed note <filename>` |
| Note ID not found (`show`, `delete`, `edit`)      | Print `Error: note '<id>' not found.` to stderr, exit 1                                                |
| `git2` fails (corrupt repo, permission error)     | Fall back gracefully: store `git_repo` and `git_branch` as `none`, continue saving the note            |
| `$EDITOR` / `$VISUAL` unset for `nota edit`       | Print actionable error message (see §6.9), exit 1                                                      |
| `~/.notes/` is not writable                       | Print `Error: cannot write to ~/.notes/ — check permissions.`, exit 1                                  |
| `nota add` receives empty input (stdin or inline) | Print `Error: note body cannot be empty.`, exit 1                                                      |

---

## 8. Technical Architecture

### 8.1 File Structure

```
nota/
├── src/
│   ├── main.rs       # CLI entry point — clap commands & dispatch
│   ├── note.rs       # Note struct, file read/write, YAML frontmatter
│   ├── context.rs    # Auto-capture: dir, git repo, branch, timestamp
│   ├── search.rs     # Full-text search + filtering logic
│   └── display.rs    # Terminal output formatting (colored tables)
├── Cargo.toml
└── README.md
```

### 8.2 Dependencies (Cargo.toml)

| Crate                  | Purpose                                          |
| ---------------------- | ------------------------------------------------ |
| `clap`                 | CLI argument parsing with derive macros          |
| `chrono`               | Timestamp generation and formatting              |
| `git2`                 | Git repository introspection (repo name, branch) |
| `serde` + `serde_yaml` | YAML frontmatter serialization/deserialization   |
| `colored`              | Terminal color output                            |
| `uuid`                 | Note ID generation (v4)                          |
| `walkdir`              | Scanning `~/.notes/` directory recursively       |

### 8.3 Storage Layout

Notes are stored in `~/.notes/` as individual `.md` files named by their 8-char ID (e.g. `~/.notes/a3f9c1bd.md`). This makes them human-browsable, grep-able, and trivially backed up or synced via any file sync tool.

---

## 9. CLI UX Design

The tool should be installed as a single binary named `nota`. Running `nota --help` should display a clean, well-formatted usage guide. Every subcommand should support `--help`.

Color usage:

- Timestamps — dim gray
- Repo / branch — blue
- Note body — white
- Matched search terms — yellow bold

All color output should be suppressed when stdout is not a TTY (i.e. when piped to another command).

---

## 10. Milestones

Dependencies are listed explicitly. No milestone should begin until its dependency is complete.

| Milestone            | Timeline | Depends On  | Scope                                                                                                                                                             |
| -------------------- | -------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| M1 — Core            | Week 1   | —           | `nota add` (inline + stdin), `nota list`, `nota show`, file storage with YAML frontmatter, git context capture, ID generation, error handling for all M1 commands |
| M2 — Search & Filter | Week 2   | M1 complete | `nota search`, `nota context` with flag support, `nota list` narrowing flags (`--here`, `--repo`, `--branch`), colored output                                     |
| M3 — Polish          | Week 3   | M2 complete | `nota log`, `nota delete`, `--help` docs for all commands, README with full usage examples and scalability note                                                   |
| M4 — Stretch         | Optional | M3 complete | `nota edit`, shell completion scripts, `brew`/`cargo install` packaging                                                                                           |

---

## 11. Success Metrics

| Metric                           | Target                                                                 |
| -------------------------------- | ---------------------------------------------------------------------- |
| `nota add` execution time        | < 50ms on cold start                                                   |
| `nota search` across 1,000 notes | < 200ms                                                                |
| Binary size                      | < 5MB release build                                                    |
| P0 features complete             | All 6 implemented and working                                          |
| Error handling coverage          | All scenarios in §7 handled gracefully, no panics in release build     |
| README quality                   | Clear install + usage instructions with examples and known limitations |
