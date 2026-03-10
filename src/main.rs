mod context;
mod display;
mod note;
mod search;

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use std::io::IsTerminal;

// ── CLI definition ────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "nota",
    about = "Contextual note-taking CLI — notes tagged with git repo, branch, and directory",
    version,
    propagate_version = true
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a note (reads from stdin if no text provided)
    Add {
        /// Note text inline
        text: Option<String>,
    },

    /// List notes, most recent first
    List {
        /// Maximum number of notes to show
        #[arg(long, default_value_t = 20)]
        limit: usize,
        /// Filter by current repo and directory
        #[arg(long)]
        here: bool,
        /// Filter by repo name
        #[arg(long)]
        repo: Option<String>,
        /// Filter by branch name
        #[arg(long)]
        branch: Option<String>,
        /// Filter by tag
        #[arg(long)]
        tag: Option<String>,
    },

    /// Print a note's full body to stdout (pipe-friendly, no frontmatter)
    Show {
        /// Note ID (8-char hex)
        id: String,
    },

    /// Search note bodies for a keyword or phrase
    Search {
        /// Search query
        query: String,
        /// Scope search to current repo and directory
        #[arg(long)]
        here: bool,
        /// Filter results by tag
        #[arg(long)]
        tag: Option<String>,
    },

    /// Show notes from current repo and directory
    Context {
        /// Filter by repo name
        #[arg(long)]
        repo: Option<String>,
        /// Filter by branch name
        #[arg(long)]
        branch: Option<String>,
    },

    /// Activity log grouped by repository
    Log {
        /// Limit to the last N days
        #[arg(long)]
        days: Option<u64>,
    },

    /// Delete a note by ID (prompts for confirmation)
    Delete {
        /// Note ID (8-char hex)
        id: String,
    },

    /// Open a note in $EDITOR
    Edit {
        /// Note ID (8-char hex)
        id: String,
    },

    /// Add or remove tags on a note
    #[command(subcommand)]
    Tag(TagCommands),

    /// Show stats about your notes
    Stats,

    /// Print shell completion script to stdout
    ///
    /// Usage: nota completions zsh >> ~/.zshrc
    ///        nota completions bash >> ~/.bashrc
    ///        nota completions fish > ~/.config/fish/completions/nota.fish
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[derive(Subcommand)]
enum TagCommands {
    /// Add one or more tags to a note
    Add {
        /// Note ID (8-char hex)
        id: String,
        /// Tags to add
        #[arg(required = true)]
        tags: Vec<String>,
    },
    /// Remove one or more tags from a note
    Rm {
        /// Note ID (8-char hex)
        id: String,
        /// Tags to remove
        #[arg(required = true)]
        tags: Vec<String>,
    },
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();

    // Suppress color globally when stdout is not a TTY
    if !std::io::stdout().is_terminal() {
        colored::control::set_override(false);
    }

    if let Err(e) = run(cli) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Commands::Add { text } => cmd_add(text),
        Commands::List {
            limit,
            here,
            repo,
            branch,
            tag,
        } => cmd_list(limit, here, repo, branch, tag),
        Commands::Show { id } => cmd_show(&id),
        Commands::Search { query, here, tag } => cmd_search(&query, here, tag),
        Commands::Context { repo, branch } => cmd_context(repo, branch),
        Commands::Log { days } => cmd_log(days),
        Commands::Delete { id } => cmd_delete(&id),
        Commands::Edit { id } => cmd_edit(&id),
        Commands::Tag(sub) => cmd_tag(sub),
        Commands::Stats => cmd_stats(),
        Commands::Completions { shell } => cmd_completions(shell),
    }
}

// ── Command handlers ──────────────────────────────────────────────────────────

fn cmd_add(text: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Read;

    let body = match text {
        Some(t) => t.trim().to_string(),
        None => {
            // Print hint only when stdin is a live terminal, not a pipe
            if std::io::stdin().is_terminal() {
                eprintln!("Enter note (Ctrl+D to save):");
            }
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            buf.trim().to_string()
        }
    };

    if body.is_empty() {
        return Err("note body cannot be empty.".into());
    }

    let ctx = context::capture_context();
    let id = note::generate_id();

    let n = note::Note {
        frontmatter: note::NoteFrontmatter {
            id: id.clone(),
            timestamp: ctx.timestamp,
            directory: ctx.directory,
            git_repo: ctx.git_repo,
            git_branch: ctx.git_branch,
            tags: Vec::new(),
            changed_files: ctx.changed_files,
            unstaged_files: ctx.unstaged_files,
            untracked_files: ctx.untracked_files,
        },
        body,
        file_path: note::notes_dir().join(format!("{}.md", id)),
    };

    note::ensure_notes_dir()
        .map_err(|e| format!("cannot write to ~/.notes/ — check permissions: {}", e))?;

    note::write_note(&n)
        .map_err(|e| format!("cannot write to ~/.notes/ — check permissions: {}", e))?;

    println!("Saved note {}.", n.frontmatter.id);
    Ok(())
}

fn cmd_list(
    limit: usize,
    here: bool,
    repo: Option<String>,
    branch: Option<String>,
    tag: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let ctx = if here {
        Some(context::capture_context())
    } else {
        None
    };

    let opts = search::FilterOptions {
        limit,
        here,
        repo,
        branch,
        tag,
        current_dir: ctx.as_ref().map(|c| c.directory.clone()),
        current_repo: ctx.as_ref().map(|c| c.git_repo.clone()),
    };

    let mut notes = search::load_all_notes(&note::notes_dir());
    search::sort_by_recency(&mut notes);
    let notes = search::apply_filters(notes, &opts);

    let config = display::DisplayConfig::default();
    display::print_notes_table(&notes, &config);
    Ok(())
}

fn cmd_show(id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let n = note::load_note_by_id(id)?;
    print!("{}", n.body);
    display::print_changed_files(
        &n.frontmatter.changed_files,
        &n.frontmatter.unstaged_files,
        &n.frontmatter.untracked_files,
    );
    Ok(())
}

fn cmd_search(
    query: &str,
    here: bool,
    tag: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let ctx = if here {
        Some(context::capture_context())
    } else {
        None
    };

    let opts = search::FilterOptions {
        here,
        tag,
        current_dir: ctx.as_ref().map(|c| c.directory.clone()),
        current_repo: ctx.as_ref().map(|c| c.git_repo.clone()),
        ..Default::default()
    };

    let mut notes = search::load_all_notes(&note::notes_dir());
    search::sort_by_recency(&mut notes);
    let notes = search::search_notes(notes, query);
    let notes = search::apply_filters(notes, &opts);

    let config = display::DisplayConfig::default();
    display::print_notes_with_highlight(&notes, query, &config);
    Ok(())
}

fn cmd_context(
    repo: Option<String>,
    branch: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let opts = if repo.is_none() && branch.is_none() {
        // No flags: match current repo + current directory (equivalent to nota list --here)
        let ctx = context::capture_context();
        search::FilterOptions {
            here: true,
            current_dir: Some(ctx.directory),
            current_repo: Some(ctx.git_repo),
            ..Default::default()
        }
    } else {
        // Explicit flags: apply only those, no directory constraint
        search::FilterOptions {
            repo,
            branch,
            ..Default::default()
        }
    };

    let mut notes = search::load_all_notes(&note::notes_dir());
    search::sort_by_recency(&mut notes);
    let notes = search::apply_filters(notes, &opts);

    let config = display::DisplayConfig::default();
    display::print_notes_table(&notes, &config);
    Ok(())
}

fn cmd_log(days: Option<u64>) -> Result<(), Box<dyn std::error::Error>> {
    let notes = search::load_all_notes(&note::notes_dir());
    let groups = search::group_by_repo(&notes, days);
    let config = display::DisplayConfig::default();
    display::print_log_table(&groups, &config);
    Ok(())
}

fn cmd_delete(id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = note::notes_dir().join(format!("{}.md", id));
    if !path.exists() {
        return Err(format!("note '{}' not found.", id).into());
    }

    eprint!("Delete note {}? [y/N] ", id);
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    if matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
        note::delete_note_by_id(id)?;
        println!("Deleted note {}.", id);
    } else {
        println!("Aborted.");
    }
    Ok(())
}

fn cmd_edit(id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let n = note::load_note_by_id(id)?;

    let editor = std::env::var("EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .map_err(|_| {
            "$EDITOR is not set. Set it in your shell profile (e.g. export EDITOR=vim)."
        })?;

    std::process::Command::new(&editor)
        .arg(&n.file_path)
        .status()
        .map_err(|e| format!("failed to launch editor '{}': {}", editor, e))?;

    Ok(())
}

fn cmd_tag(sub: TagCommands) -> Result<(), Box<dyn std::error::Error>> {
    match sub {
        TagCommands::Add { id, tags } => {
            let mut n = note::load_note_by_id(&id)?;
            let mut changed = false;
            for tag in &tags {
                let tag = tag.trim().to_lowercase();
                if tag.is_empty() {
                    continue;
                }
                if !n.frontmatter.tags.iter().any(|t| t.to_lowercase() == tag) {
                    n.frontmatter.tags.push(tag);
                    changed = true;
                }
            }
            if changed {
                note::write_note(&n)?;
                println!("Tagged note {} with: {}", id, n.frontmatter.tags.join(", "));
            } else {
                println!("No new tags added (already present).");
            }
        }
        TagCommands::Rm { id, tags } => {
            let mut n = note::load_note_by_id(&id)?;
            let before = n.frontmatter.tags.len();
            let remove: Vec<String> = tags.iter().map(|t| t.to_lowercase()).collect();
            n.frontmatter
                .tags
                .retain(|t| !remove.contains(&t.to_lowercase()));
            let removed = before - n.frontmatter.tags.len();
            if removed > 0 {
                note::write_note(&n)?;
                println!(
                    "Removed {} tag(s). Remaining: {}",
                    removed,
                    if n.frontmatter.tags.is_empty() {
                        "none".to_string()
                    } else {
                        n.frontmatter.tags.join(", ")
                    }
                );
            } else {
                println!("No matching tags found.");
            }
        }
    }
    Ok(())
}

fn cmd_stats() -> Result<(), Box<dyn std::error::Error>> {
    let notes = search::load_all_notes(&note::notes_dir());
    let stats = search::compute_stats(&notes);
    let config = display::DisplayConfig::default();
    display::print_stats(&stats);
    let _ = config; // DisplayConfig reserved for future use
    Ok(())
}

fn cmd_completions(shell: Shell) -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Cli::command();
    let name = cmd.get_name().to_string();
    clap_complete::generate(shell, &mut cmd, name, &mut std::io::stdout());
    Ok(())
}
