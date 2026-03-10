# nota — Contextual Note Taker

A fast CLI note-taking tool that automatically captures context (git repo, branch, directory, timestamp) so you can find notes by *where you were*, not just *what you wrote*.

## Features

- **Context capture** — Every note records your current git repo, branch, directory, and timestamp
- **Fast retrieval** — Find notes by project, branch, or time with `nota search`, `nota context`, and `nota list`
- **Plain text** — Notes are stored as readable Markdown files in `~/.notes/`
- **Zero friction** — Save a note in one command: `nota add "your note here"`

## Quick Start

```bash
# Add a note
nota add "Fix the auth middleware tomorrow"

# List recent notes
nota list

# Search for a note
nota search "middleware"

# Show notes from current repo
nota context

# View a single note
nota show a3f9c1bd
```

## Install

```bash
cargo install --path .
```

## Project Status

Currently in development. See `notaPRD.md` for the full specification.

## Architecture

- **src/main.rs** — CLI entry point and command dispatch
- **src/note.rs** — Note struct and file I/O with YAML frontmatter
- **src/context.rs** — Git and directory context capture
- **src/search.rs** — Search and filtering logic
- **src/display.rs** — Terminal formatting
