use crate::note::Note;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Terminal;
use std::error::Error;
use std::io;

// ── App state ─────────────────────────────────────────────────────────────────

pub struct App {
    notes: Vec<Note>,
    selected: usize,
    search_query: String,
    search_mode: bool,
    tag_input_mode: bool,
    tag_input: String,
    filtered_indices: Vec<usize>,
    message: String,
    message_timeout: u16,
}

impl App {
    pub fn new(notes: Vec<Note>) -> Self {
        let filtered_indices: Vec<usize> = (0..notes.len()).collect();
        Self {
            notes,
            selected: 0,
            search_query: String::new(),
            search_mode: false,
            tag_input_mode: false,
            tag_input: String::new(),
            filtered_indices,
            message: String::new(),
            message_timeout: 0,
        }
    }

    fn set_message(&mut self, msg: &str) {
        self.message = msg.to_string();
        self.message_timeout = 60; // show for ~3 seconds at 20fps
    }

    fn tick(&mut self) {
        if self.message_timeout > 0 {
            self.message_timeout -= 1;
        }
    }

    /// Get the currently selected note
    fn selected_note(&self) -> Option<&Note> {
        if self.filtered_indices.is_empty() {
            return None;
        }
        self.filtered_indices
            .get(self.selected)
            .and_then(|&idx| self.notes.get(idx))
    }

    /// Update filtered indices based on search query
    fn update_filter(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_indices = (0..self.notes.len()).collect();
        } else {
            self.filtered_indices = self
                .notes
                .iter()
                .enumerate()
                .filter_map(|(idx, note)| {
                    if note.body.to_lowercase().contains(&self.search_query.to_lowercase()) {
                        Some(idx)
                    } else {
                        None
                    }
                })
                .collect();
        }
        self.selected = 0;
    }

    /// Handle keyboard input
    fn handle_input(&mut self, key: KeyEvent) -> bool {
        if self.tag_input_mode {
            return self.handle_tag_input(key);
        }

        if self.search_mode {
            return self.handle_search_input(key);
        }

        match (key.code, key.modifiers) {
            // Navigation
            (KeyCode::Up, _) | (KeyCode::Char('k'), KeyModifiers::NONE) => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
            }
            (KeyCode::Down, _) | (KeyCode::Char('j'), KeyModifiers::NONE) => {
                if self.selected < self.filtered_indices.len().saturating_sub(1) {
                    self.selected += 1;
                }
            }
            (KeyCode::Home, _) => self.selected = 0,
            (KeyCode::End, _) => {
                self.selected = self.filtered_indices.len().saturating_sub(1);
            }
            // Search
            (KeyCode::Char('/'), _) => {
                self.search_mode = true;
                self.search_query.clear();
            }
            // Tag mode
            (KeyCode::Char('t'), _) => {
                if !self.filtered_indices.is_empty() {
                    self.tag_input_mode = true;
                    self.tag_input.clear();
                    self.set_message("Enter tag (space-separated):");
                }
            }
            // Quit
            (KeyCode::Char('q'), _) | (KeyCode::Esc, _) => return false,
            _ => {}
        }
        true
    }

    fn handle_tag_input(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Enter => {
                // Add tags to the selected note
                if !self.tag_input.is_empty() && !self.filtered_indices.is_empty() {
                    let note_idx = self.filtered_indices[self.selected];
                    let note_id = self.notes[note_idx].frontmatter.id.clone();
                    
                    let note = &mut self.notes[note_idx];
                    for tag in self.tag_input.split_whitespace() {
                        let tag_lower = tag.to_lowercase();
                        if !note.frontmatter.tags.contains(&tag_lower) {
                            note.frontmatter.tags.push(tag_lower);
                        }
                    }
                    
                    self.set_message(&format!("Added tags to note {}", note_id));
                }
                self.tag_input_mode = false;
            }
            KeyCode::Esc => {
                self.tag_input_mode = false;
                self.tag_input.clear();
                self.message.clear();
            }
            KeyCode::Backspace => {
                self.tag_input.pop();
            }
            KeyCode::Char(c) => {
                self.tag_input.push(c);
            }
            _ => {}
        }
        true
    }

    /// Handle input while in search mode
    fn handle_search_input(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Enter => {
                self.search_mode = false;
            }
            KeyCode::Esc => {
                self.search_mode = false;
                self.search_query.clear();
                self.update_filter();
            }
            KeyCode::Backspace => {
                self.search_query.pop();
                self.update_filter();
            }
            KeyCode::Char(c) => {
                self.search_query.push(c);
                self.update_filter();
            }
            _ => {}
        }
        true
    }
}

// ── TUI rendering ─────────────────────────────────────────────────────────────

pub fn run_tui(notes: Vec<Note>) -> Result<(), Box<dyn Error>> {
    let mut terminal = setup_terminal()?;
    let app = App::new(notes);
    let result = run_app(&mut terminal, app);
    restore_terminal(&mut terminal)?;

    result
}

fn setup_terminal() -> Result<Terminal<ratatui::prelude::CrosstermBackend<io::Stdout>>, Box<dyn Error>> {
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
    let backend = ratatui::prelude::CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal(
    terminal: &mut Terminal<ratatui::prelude::CrosstermBackend<io::Stdout>>,
) -> Result<(), Box<dyn Error>> {
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;
    Ok(())
}

fn run_app(
    terminal: &mut Terminal<ratatui::prelude::CrosstermBackend<io::Stdout>>,
    mut app: App,
) -> Result<(), Box<dyn Error>> {
    loop {
        terminal.draw(|f| ui(f, &app))?;

        if crossterm::event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if !app.handle_input(key) {
                    break;
                }
            }
        }
        
        app.tick();
    }

    Ok(())
}

// ── UI components ─────────────────────────────────────────────────────────────

fn ui(f: &mut ratatui::Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(f.size());

    // Left panel: notes list
    render_list(f, app, chunks[0]);

    // Right panel: note preview
    render_preview(f, app, chunks[1]);
}

fn render_list(f: &mut ratatui::Frame, app: &App, area: ratatui::layout::Rect) {
    let items: Vec<ListItem> = app
        .filtered_indices
        .iter()
        .enumerate()
        .map(|(idx, &note_idx)| {
            let note = &app.notes[note_idx];
            let first_line = note.body.lines().next().unwrap_or("(empty)");
            let truncated = if first_line.len() > 35 {
                format!("{}…", &first_line[..35])
            } else {
                first_line.to_string()
            };

            let style = if idx == app.selected {
                Style::default()
                    .fg(Color::White)
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(truncated).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Notes ")
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Cyan)),
        )
        .style(Style::default());

    f.render_widget(list, area);
}

fn render_preview(f: &mut ratatui::Frame, app: &App, area: ratatui::layout::Rect) {
    let mut content = vec![];

    if let Some(note) = app.selected_note() {
        content.push(Line::from(vec![
            Span::styled("ID: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(&note.frontmatter.id),
        ]));

        content.push(Line::from(""));

        content.push(Line::from(vec![
            Span::styled("Timestamp: ", Style::default().fg(Color::Cyan)),
            Span::raw(&note.frontmatter.timestamp),
        ]));

        content.push(Line::from(vec![
            Span::styled("Repo: ", Style::default().fg(Color::Cyan)),
            Span::raw(&note.frontmatter.git_repo),
        ]));

        if note.frontmatter.commit_hash != "none" {
            let short_hash = &note.frontmatter.commit_hash[..8.min(note.frontmatter.commit_hash.len())];
            content.push(Line::from(vec![
                Span::styled("Commit: ", Style::default().fg(Color::Cyan)),
                Span::raw(short_hash),
            ]));
        }

        if !note.frontmatter.tags.is_empty() {
            content.push(Line::from(vec![
                Span::styled("Tags: ", Style::default().fg(Color::Cyan)),
                Span::raw(note.frontmatter.tags.join(", ")),
            ]));
        }

        content.push(Line::from(""));
        content.push(Line::from(
            Span::styled("─── Body ───", Style::default().fg(Color::Gray))
        ));
        content.push(Line::from(""));

        for line in note.body.lines() {
            content.push(Line::from(line));
        }
    } else {
        content.push(Line::from(
            Span::styled("No notes", Style::default().fg(Color::Red))
        ));
    }

    let mut help_text = vec![
        "",
        "─── Keybinds ───",
        "j/k, ↑/↓  Navigate",
        "/         Search",
        "t         Add tags",
        "q, Esc    Quit",
    ];

    if app.search_mode {
        help_text = vec!["", "Search mode: Type to filter, Enter/Esc to exit"];
    }

    if app.tag_input_mode {
        help_text = vec!["", "Tag mode: Type tags (space-separated), Enter to add, Esc to cancel"];
    }

    let mut help_lines: Vec<Line> = help_text.iter().map(|l| Line::from(*l)).collect();
    
    // Add message if visible
    if !app.message.is_empty() && app.message_timeout > 0 {
        help_lines.insert(0, Line::from(vec![
            Span::styled(&app.message, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        ]));
        help_lines.insert(1, Line::from(""));
    }

    if app.tag_input_mode {
        help_lines.push(Line::from(vec![
            Span::raw("> "),
            Span::styled(&app.tag_input, Style::default().fg(Color::Yellow))
        ]));
    }

    let paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .title(" Note Preview ")
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Cyan)),
        )
        .alignment(Alignment::Left);

    f.render_widget(paragraph, area);

    // Show help at bottom
    let help_area = ratatui::layout::Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(help_lines.len() as u16 + 1),
        width: area.width,
        height: (help_lines.len() as u16).min(10),
    };

    let help_para = Paragraph::new(help_lines)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Left);

    f.render_widget(help_para, help_area);
}
