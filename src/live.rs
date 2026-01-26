// Live Search TUI

use anyhow::Result;
use crossterm::{
	event::{self, Event, KeyCode, KeyEventKind},
	execute,
	terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
	layout::{Constraint, Direction, Layout},
	style::{Color, Modifier, Style},
	text::{Line, Span},
	widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
	Terminal,
};
use std::{
	io,
	path::{Path, PathBuf},
	time::{Duration, Instant},
};

use crate::config::DEBOUNCE_TIME_MS;
use crate::embedder::TextEncoder;
use crate::embedding::cosine_similarity;
use crate::sidecar::{current_version, iter_sidecars, ImageSidecar};

struct CachedImage {
	path: PathBuf,
	embedding: Vec<f32>,
}

struct AppState {
	query: String,
	cursor_visible: bool,
	results: Vec<(PathBuf, f32)>,
	selected: usize,
	index: Vec<CachedImage>,
	encoder: TextEncoder,
	status: String,
}

impl AppState {
	fn select_next(&mut self) {
		if !self.results.is_empty() {
			self.selected = (self.selected + 1).min(self.results.len() - 1);
		}
	}

	fn select_prev(&mut self) {
		if self.selected > 0 {
			self.selected -= 1;
		}
	}
}

pub fn run_live_search(directory: &Path, recursive: bool) -> Result<()> {
	enable_raw_mode()?;
	let mut stdout = io::stdout();
	execute!(stdout, EnterAlternateScreen)?;
	let backend = ratatui::backend::CrosstermBackend::new(stdout);
	let mut terminal = Terminal::new(backend)?;

	let encoder = match TextEncoder::new() {
		Ok(e) => e,
		Err(e) => {
			cleanup_terminal()?;
			return Err(e);
		}
	};

	let mut app = AppState {
		query: String::new(),
		cursor_visible: true,
		results: Vec::new(),
		selected: 0,
		index: Vec::new(),
		encoder,
		status: "Loading index...".to_string(),
	};

	terminal.draw(|f| ui(f, &mut app))?;

	let root = directory.canonicalize().unwrap_or_else(|_| directory.to_path_buf());
	let mut loaded = 0;
	let mut outdated = 0;

	for (sidecar_path, base_dir) in iter_sidecars(&root, recursive) {
		if let Ok(sidecar) = ImageSidecar::load(&sidecar_path) {
			if !sidecar.is_current_version() {
				outdated += 1;
			}
			let full_path = base_dir.join(&sidecar.filename);
			app.index.push(CachedImage {
				path: full_path,
				embedding: sidecar.embedding,
			});
			loaded += 1;

			if loaded % 100 == 0 {
				app.status = format!("Loading: {}...", loaded);
				terminal.draw(|f| ui(f, &mut app))?;
			}
		}
	}

	if outdated > 0 {
		app.status = format!(
			"Ready. {} indexed ({} outdated, run 'scout scan -f' to upgrade to v{})",
			loaded, outdated, current_version()
		);
	} else {
		app.status = format!("Ready. {} images indexed.", loaded);
	}

	let mut last_input = Instant::now();
	let mut last_query = String::new();
	let mut last_blink = Instant::now();
	let debounce = Duration::from_millis(DEBOUNCE_TIME_MS);
	let blink_rate = Duration::from_millis(530);

	loop {
		if last_blink.elapsed() >= blink_rate {
			app.cursor_visible = !app.cursor_visible;
			last_blink = Instant::now();
		}

		terminal.draw(|f| ui(f, &mut app))?;

		if event::poll(Duration::from_millis(50))? {
			if let Event::Key(key) = event::read()? {
				if key.kind == KeyEventKind::Press {
					app.cursor_visible = true;
					last_blink = Instant::now();

					match key.code {
						KeyCode::Esc => break,
						KeyCode::Char(c) => {
							app.query.push(c);
							last_input = Instant::now();
						}
						KeyCode::Backspace => {
							app.query.pop();
							last_input = Instant::now();
						}
						KeyCode::Enter => {
							perform_search(&mut app);
							last_query = app.query.clone();
						}
						KeyCode::Down | KeyCode::Tab => app.select_next(),
						KeyCode::Up | KeyCode::BackTab => app.select_prev(),
						_ => {}
					}
				}
			}
		}

		if last_input.elapsed() > debounce && app.query != last_query {
			if !app.query.is_empty() {
				app.status = "Searching...".to_string();
				terminal.draw(|f| ui(f, &mut app))?;
				perform_search(&mut app);
			} else {
				app.results.clear();
				app.selected = 0;
				app.status = format!("Ready. {} images indexed.", app.index.len());
			}
			last_query = app.query.clone();
		}
	}

	cleanup_terminal()?;
	Ok(())
}

fn perform_search(app: &mut AppState) {
	let start = Instant::now();

	let query_emb = match app.encoder.embed(&app.query) {
		Ok(e) => e,
		Err(e) => {
			app.status = format!("Error: {}", e);
			return;
		}
	};

	let mut scores: Vec<(PathBuf, f32)> = app
		.index
		.iter()
		.map(|img| (img.path.clone(), cosine_similarity(&query_emb, &img.embedding)))
		.filter(|(_, s)| *s > 0.0)
		.collect();

	scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
	app.results = scores.into_iter().take(50).collect();
	app.selected = 0;

	let ms = start.elapsed().as_millis();
	app.status = format!("{} matches in {}ms", app.results.len(), ms);
}

fn cleanup_terminal() -> Result<()> {
	disable_raw_mode()?;
	execute!(io::stdout(), LeaveAlternateScreen)?;
	Ok(())
}

fn ui(f: &mut ratatui::Frame, app: &mut AppState) {
	let main_chunks = Layout::default()
		.direction(Direction::Vertical)
		.constraints([
			Constraint::Length(3),
			Constraint::Min(1),
			Constraint::Length(1),
		])
		.split(f.area());

	// Search bar with blinking cursor
	let cursor = if app.cursor_visible { "|" } else { " " };
	let search_text = if app.query.is_empty() {
		Line::from(vec![
			Span::styled(" 🔍 ", Style::default()),
			Span::styled("Type to search...", Style::default().fg(Color::DarkGray)),
			Span::styled(cursor, Style::default().fg(Color::Cyan)),
		])
	} else {
		Line::from(vec![
			Span::styled(" 🔍 ", Style::default()),
			Span::styled(&app.query, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
			Span::styled(cursor, Style::default().fg(Color::Cyan)),
		])
	};

	let search_block = Paragraph::new(search_text).block(
		Block::default()
			.borders(Borders::ALL)
			.title(" Search ")
			.border_style(Style::default().fg(Color::Blue)),
	);
	f.render_widget(search_block, main_chunks[0]);

	render_results(f, app, main_chunks[1]);

	let status = Paragraph::new(app.status.as_str()).style(Style::default().fg(Color::DarkGray));
	f.render_widget(status, main_chunks[2]);
}

fn render_results(f: &mut ratatui::Frame, app: &AppState, area: ratatui::layout::Rect) {
	let items: Vec<ListItem> = app
		.results
		.iter()
		.enumerate()
		.map(|(i, (path, score))| {
			let filename = path.file_name().unwrap_or_default().to_string_lossy();
			let score_pct = (score * 100.0) as u32;
			let is_selected = i == app.selected;
			let prefix = if is_selected { "▶ " } else { "  " };

			let score_style = if score_pct > 20 {
				Style::default().fg(Color::Green)
			} else if score_pct > 10 {
				Style::default().fg(Color::Yellow)
			} else {
				Style::default().fg(Color::DarkGray)
			};

			let name_style = if is_selected {
				Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
			} else {
				Style::default().fg(Color::White)
			};

			ListItem::new(Line::from(vec![
				Span::styled(prefix, Style::default().fg(Color::Cyan)),
				Span::styled(format!("{:3}% ", score_pct), score_style),
				Span::styled(filename.to_string(), name_style),
			]))
		})
		.collect();

	let mut state = ListState::default();
	state.select(Some(app.selected));

	let list = List::new(items)
		.block(Block::default().borders(Borders::ALL).title(" Results "))
		.highlight_style(Style::default().bg(Color::DarkGray));

	f.render_stateful_widget(list, area, &mut state);
}