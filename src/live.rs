//! Interactive terminal UI for real-time image search
//!
//! Provides a TUI with search-as-you-type functionality, result navigation,
//! and file metadata display.

use anyhow::Result;
use crossterm::{
	cursor,
	event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
	execute,
	terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
	backend::CrosstermBackend,
	layout::{Constraint, Direction, Layout, Rect},
	style::{Color, Modifier, Style},
	symbols,
	text::{Line, Span},
	widgets::{Block, Borders, List, ListItem, Paragraph},
	Terminal,
};
use std::{
	fs,
	io::{self, Write},
	path::{Path, PathBuf},
	time::{Duration, Instant, SystemTime},
};

use crate::config::{CURSOR_BLINK_MS, DEBOUNCE_MS, LIVE_INDEX_PROGRESS, LIVE_RESULTS_LIMIT, SCORE_HIGH, SCORE_MED};
use crate::models::ModelManager;
use crate::sidecar::{iter_sidecars, Sidecar};
use crate::types::Embedding;

struct IndexedMedia {
	path: PathBuf,
	/// For images: single (None, embedding)
	/// For videos: Vec of (Some(timestamp), embedding) per frame
	frames: Vec<(Option<f64>, Embedding)>,
}

struct FileInfo {
	resolution: Option<(u32, u32)>,
	size_bytes: u64,
	modified: Option<SystemTime>,
}

impl FileInfo {
	fn load(path: &Path) -> Self {
		let metadata = fs::metadata(path).ok();
		let size_bytes = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
		let modified = metadata.as_ref().and_then(|m| m.modified().ok());

		let resolution = image::ImageReader::open(path)
			.ok()
			.and_then(|r| r.into_dimensions().ok());

		Self { resolution, size_bytes, modified }
	}

	fn size_display(&self) -> String {
		if self.size_bytes >= 1024 * 1024 {
			format!("{:.1}MB", self.size_bytes as f64 / (1024.0 * 1024.0))
		} else if self.size_bytes >= 1024 {
			format!("{:.1}KB", self.size_bytes as f64 / 1024.0)
		} else {
			format!("{}B", self.size_bytes)
		}
	}

	fn resolution_display(&self) -> Option<String> {
		self.resolution.map(|(w, h)| format!("{}×{}", w, h))
	}

	fn date_display(&self) -> Option<String> {
		self.modified.map(|t| {
			let dt: chrono::DateTime<chrono::Utc> = t.into();
			dt.format("%Y-%m-%d").to_string()
		})
	}
}

struct App {
	query: String,
	cursor_visible: bool,
	results: Vec<(PathBuf, f32, Option<f64>)>, // (path, score, timestamp)
	selected: usize,
	list_offset: usize,
	index: Vec<IndexedMedia>,
	models: ModelManager,
	status: String,
	status_type: StatusType,
	file_info: Option<FileInfo>,
	info_pending: bool,
	last_info_path: Option<PathBuf>,
}

#[derive(Clone, Copy, PartialEq)]
enum StatusType {
	Normal,
	Success,
	Warning,
	Loading,
}

impl App {
	fn new(models: ModelManager) -> Self {
		Self {
			query: String::new(),
			cursor_visible: true,
			results: Vec::new(),
			selected: 0,
			list_offset: 0,
			index: Vec::new(),
			models,
			status: "Loading index...".into(),
			status_type: StatusType::Loading,
			file_info: None,
			info_pending: false,
			last_info_path: None,
		}
	}

	fn visible_count(&self, results_height: u16) -> usize {
		results_height.saturating_sub(2) as usize
	}

	fn select_next(&mut self, visible: usize) {
		if !self.results.is_empty() {
			let max = self.results.len().saturating_sub(1);
			self.selected = (self.selected + 1).min(max);
			self.adjust_scroll(visible);
			self.mark_info_pending();
		}
	}

	fn select_prev(&mut self, visible: usize) {
		if self.selected > 0 {
			self.selected -= 1;
			self.adjust_scroll(visible);
			self.mark_info_pending();
		}
	}

	fn adjust_scroll(&mut self, visible: usize) {
		if visible == 0 {
			return;
		}
		if self.selected < self.list_offset {
			self.list_offset = self.selected;
		} else if self.selected >= self.list_offset + visible {
			self.list_offset = self.selected.saturating_sub(visible) + 1;
		}
	}

	fn mark_info_pending(&mut self) {
		let current = self.results.get(self.selected).map(|(p, _, _)| p.clone());
		if current != self.last_info_path {
			self.info_pending = true;
		}
	}

	fn update_file_info(&mut self) {
		let path = self.results.get(self.selected).map(|(p, _, _)| p.clone());

		if let Some(path) = path {
			if Some(&path) != self.last_info_path.as_ref() {
				self.file_info = Some(FileInfo::load(&path));
				self.last_info_path = Some(path);
			}
		} else {
			self.file_info = None;
			self.last_info_path = None;
		}
		self.info_pending = false;
	}

	fn open_selected(&self) {
		if let Some((path, _, _)) = self.results.get(self.selected) {
			let _ = open::that(path);
		}
	}

	fn search(&mut self) {
		self.selected = 0;
		self.list_offset = 0;

		if self.query.is_empty() {
			self.results.clear();
			self.file_info = None;
			self.last_info_path = None;
			self.status = format!("{} items indexed", self.index.len());
			self.status_type = StatusType::Normal;
			return;
		}

		let start = Instant::now();

		let query_emb = match self.models.encode_text(&self.query) {
			Ok(emb) => emb,
			Err(e) => {
				self.status = format!("Encode error: {}", e);
				self.status_type = StatusType::Warning;
				return;
			}
		};

		let mut scores: Vec<(PathBuf, f32, Option<f64>)> = Vec::new();

		for media in &self.index {
			// Find the best matching frame for this media item
			let mut best_score = 0.0f32;
			let mut best_timestamp = None;

			for (timestamp, embedding) in &media.frames {
				let score = query_emb.similarity(embedding);
				if score > best_score {
					best_score = score;
					best_timestamp = *timestamp;
				}
			}

			if best_score > 0.0 {
				scores.push((media.path.clone(), best_score, best_timestamp));
			}
		}

		scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

		self.results = scores.into_iter().take(LIVE_RESULTS_LIMIT).collect();

		let ms = start.elapsed().as_millis();
		self.status = format!("{} matches in {}ms", self.results.len(), ms);
		self.status_type = StatusType::Success;

		self.mark_info_pending();
	}
}

pub fn run(directory: &Path, recursive: bool) -> Result<()> {
	let mut stdout = io::stdout();

	execute!(
		stdout,
		Clear(ClearType::All),
		Clear(ClearType::Purge),
		cursor::MoveTo(0, 0)
	)?;
	stdout.flush()?;

	enable_raw_mode()?;
	execute!(stdout, EnterAlternateScreen)?;

	let backend = CrosstermBackend::new(stdout);
	let mut terminal = Terminal::new(backend)?;
	terminal.clear()?;

	let models = match ModelManager::with_text() {
		Ok(m) => m,
		Err(e) => {
			cleanup_terminal()?;
			return Err(e);
		}
	};

	let mut app = App::new(models);

	terminal.draw(|f| draw(f, &mut app))?;

	let root = directory.canonicalize().unwrap_or_else(|_| directory.to_path_buf());
	let mut loaded = 0;
	let mut outdated = 0;

	for (sidecar_path, base_dir) in iter_sidecars(&root, recursive) {
		match Sidecar::load_auto(&sidecar_path) {
			Ok(Sidecar::Image(sidecar)) => {
				if !sidecar.is_current_version() {
					outdated += 1;
				}
				let source_path = base_dir.join(&sidecar.filename);
				if source_path.exists() {
					app.index.push(IndexedMedia {
						path: source_path,
						frames: vec![(None, sidecar.embedding())],
					});
					loaded += 1;

					if loaded % LIVE_INDEX_PROGRESS == 0 {
						app.status = format!("Loading... {} items", loaded);
						terminal.draw(|f| draw(f, &mut app))?;
					}
				}
			}
			#[cfg(feature = "video")]
			Ok(Sidecar::Video(sidecar)) => {
				if !sidecar.is_current_version() {
					outdated += 1;
				}
				let source_path = base_dir.join(&sidecar.filename);
				if source_path.exists() {
					app.index.push(IndexedMedia {
						path: source_path,
						frames: sidecar
							.frames
							.iter()
							.map(|f| (Some(f.timestamp_secs), f.embedding.clone()))
							.collect(),
					});
					loaded += 1;

					if loaded % LIVE_INDEX_PROGRESS == 0 {
						app.status = format!("Loading... {} items", loaded);
						terminal.draw(|f| draw(f, &mut app))?;
					}
				}
			}
			_ => {}
		}
	}

	app.status = if outdated > 0 {
		format!("{} items ({} outdated, run scan -f)", loaded, outdated)
	} else {
		format!("{} items indexed", loaded)
	};
	app.status_type = StatusType::Normal;

	let mut last_input = Instant::now();
	let mut last_query = String::new();
	let mut last_blink = Instant::now();
	let mut last_info_check = Instant::now();
	let mut needs_redraw = true;
	let mut results_height: u16 = 10;

	let debounce = Duration::from_millis(DEBOUNCE_MS);
	let blink_rate = Duration::from_millis(CURSOR_BLINK_MS);

	loop {
		let now = Instant::now();

		if last_blink.elapsed() >= blink_rate {
			app.cursor_visible = !app.cursor_visible;
			last_blink = now;
			needs_redraw = true;
		}

		if app.info_pending && last_info_check.elapsed() >= debounce {
			app.update_file_info();
			last_info_check = now;
			needs_redraw = true;
		}

		if needs_redraw {
			terminal.draw(|f| {
				results_height = f.area().height.saturating_sub(4);
				draw(f, &mut app);
			})?;
			needs_redraw = false;
		}

		if event::poll(Duration::from_millis(50))? {
			let visible = app.visible_count(results_height);

			match event::read()? {
				Event::Key(key) if key.kind == KeyEventKind::Press => {
					last_input = now;
					needs_redraw = true;

					match key.code {
						KeyCode::Esc => break,
						KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
						KeyCode::Enter => {
							app.open_selected();
						}
						KeyCode::Up => app.select_prev(visible),
						KeyCode::Down => app.select_next(visible),
						KeyCode::PageUp => {
							for _ in 0..visible {
								app.select_prev(visible);
							}
						}
						KeyCode::PageDown => {
							for _ in 0..visible {
								app.select_next(visible);
							}
						}
						KeyCode::Home => {
							if !app.results.is_empty() {
								app.selected = 0;
								app.list_offset = 0;
								app.mark_info_pending();
							}
						}
						KeyCode::End => {
							if !app.results.is_empty() {
								app.selected = app.results.len().saturating_sub(1);
								app.adjust_scroll(visible);
								app.mark_info_pending();
							}
						}
						KeyCode::Backspace => {
							app.query.pop();
						}
						KeyCode::Char(c) => {
							app.query.push(c);
						}
						_ => {}
					}
					last_info_check = now;
				}
				Event::Resize(_, _) => {
					needs_redraw = true;
				}
				_ => {}
			}
		}

		if last_input.elapsed() > debounce && app.query != last_query {
			app.search();
			last_query = app.query.clone();
			last_info_check = now;
			needs_redraw = true;
		}
	}

	cleanup_terminal()?;
	Ok(())
}

fn cleanup_terminal() -> Result<()> {
	disable_raw_mode()?;
	execute!(
		io::stdout(),
		LeaveAlternateScreen,
		Clear(ClearType::All),
		Clear(ClearType::Purge),
		cursor::MoveTo(0, 0)
	)?;
	Ok(())
}

fn rounded_block(title: &str) -> Block<'_> {
	Block::default()
		.borders(Borders::ALL)
		.border_set(symbols::border::ROUNDED)
		.border_style(Style::default().fg(Color::White))
		.title_style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD))
		.title(format!(" {} ", title))
}

fn draw(f: &mut ratatui::Frame, app: &mut App) {
	let size = f.area();

	let outer = Layout::default()
		.direction(Direction::Vertical)
		.constraints([
			Constraint::Length(3),
			Constraint::Min(1),
			Constraint::Length(1),
		])
		.split(size);

	draw_search_box(f, app, outer[0]);
	draw_results(f, app, outer[1]);
	draw_status(f, app, outer[2]);
}

fn draw_search_box(f: &mut ratatui::Frame, app: &App, area: Rect) {
	let cursor = if app.cursor_visible { "│" } else { " " };

	let line = if app.query.is_empty() {
		Line::from(vec![
			Span::styled(" ", Style::default()),
			Span::styled(cursor, Style::default().fg(Color::Cyan)),
			Span::styled("Type to search...", Style::default().fg(Color::DarkGray)),
		])
	} else {
		Line::from(vec![
			Span::styled(" ", Style::default()),
			Span::styled(&app.query, Style::default().fg(Color::White)),
			Span::styled(cursor, Style::default().fg(Color::Cyan)),
		])
	};

	let block = rounded_block("Search");
	let widget = Paragraph::new(line).block(block);
	f.render_widget(widget, area);
}

fn draw_results(f: &mut ratatui::Frame, app: &mut App, area: Rect) {
	let inner_height = area.height.saturating_sub(2) as usize;

	// Clamp list_offset to valid range
	if !app.results.is_empty() {
		let max_offset = app.results.len().saturating_sub(1);
		app.list_offset = app.list_offset.min(max_offset);

		// Ensure selected is visible
		if app.selected >= app.list_offset + inner_height && inner_height > 0 {
			app.list_offset = app.selected.saturating_sub(inner_height) + 1;
		}
	}

	let items: Vec<ListItem> = app
		.results
		.iter()
		.enumerate()
		.skip(app.list_offset)
		.take(inner_height)
		.map(|(i, (path, score, timestamp))| {
			let selected = i == app.selected;
			let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");

			let score_color = if *score >= SCORE_HIGH {
				Color::Green
			} else if *score >= SCORE_MED {
				Color::Yellow
			} else {
				Color::DarkGray
			};

			let pointer = if selected { "›" } else { " " };
			let pointer_style = if selected {
				Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
			} else {
				Style::default()
			};

			let name_style = if selected {
				Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
			} else {
				Style::default().fg(Color::White)
			};

			let mut spans = vec![
				Span::styled(pointer, pointer_style),
				Span::styled(" ", Style::default()),
				Span::styled(filename, name_style),
				Span::styled("  ", Style::default()),
			];

			// Add timestamp for videos
			if let Some(ts) = timestamp {
				let minutes = (ts / 60.0) as u32;
				let seconds = (ts % 60.0) as u32;
				let time_str = format!("{}:{:02}  ", minutes, seconds);
				spans.push(Span::styled(time_str, Style::default().fg(Color::Blue)));
			}

			spans.push(Span::styled(format!("{:.1}%", score * 100.0), Style::default().fg(score_color)));

			let line = Line::from(spans);

			let style = if selected {
				Style::default().bg(Color::DarkGray)
			} else {
				Style::default()
			};

			ListItem::new(line).style(style)
		})
		.collect();

	let title = if app.results.is_empty() {
		"Results".into()
	} else {
		format!("Results ({}/{})", app.selected + 1, app.results.len())
	};

	let block = rounded_block(&title);
	let list = List::new(items).block(block);

	f.render_widget(list, area);
}

fn draw_status(f: &mut ratatui::Frame, app: &App, area: Rect) {
	let width = area.width as usize;

	let color = match app.status_type {
		StatusType::Normal => Color::DarkGray,
		StatusType::Success => Color::Green,
		StatusType::Warning => Color::Yellow,
		StatusType::Loading => Color::Blue,
	};

	let mut spans = vec![
		Span::styled(" ", Style::default()),
		Span::styled(&app.status, Style::default().fg(color)),
	];

	// Add file info if we have it and enough space
	if let Some(info) = &app.file_info {
		let mut info_parts: Vec<String> = Vec::new();

		if let Some(res) = info.resolution_display() {
			info_parts.push(res);
		}
		info_parts.push(info.size_display());
		if let Some(date) = info.date_display() {
			info_parts.push(date);
		}

		// Calculate how much space we have
		let base_len = app.status.len() + 40; // status + nav hints
		let available = width.saturating_sub(base_len);

		if available > 10 {
			let mut info_str = String::new();
			for (i, part) in info_parts.iter().enumerate() {
				let addition = if i == 0 { part.len() } else { part.len() + 3 };
				if info_str.len() + addition <= available {
					if i > 0 {
						info_str.push_str(" · ");
					}
					info_str.push_str(part);
				} else {
					break;
				}
			}

			if !info_str.is_empty() {
				spans.push(Span::styled("  │  ", Style::default().fg(Color::DarkGray)));
				spans.push(Span::styled(info_str, Style::default().fg(Color::White)));
			}
		}
	}

	spans.push(Span::styled("  │  ", Style::default().fg(Color::DarkGray)));
	spans.push(Span::styled("↑↓", Style::default().fg(Color::Blue)));
	spans.push(Span::styled(" navigate  ", Style::default().fg(Color::DarkGray)));
	spans.push(Span::styled("Enter", Style::default().fg(Color::Blue)));
	spans.push(Span::styled(" open  ", Style::default().fg(Color::DarkGray)));
	spans.push(Span::styled("Esc", Style::default().fg(Color::Blue)));
	spans.push(Span::styled(" quit", Style::default().fg(Color::DarkGray)));

	let line = Line::from(spans);
	let widget = Paragraph::new(line);
	f.render_widget(widget, area);
}
