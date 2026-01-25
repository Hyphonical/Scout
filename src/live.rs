// Live Search TUI for Image Sidecars

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
	widgets::{Block, Borders, List, ListItem, Paragraph},
	Terminal,
};
use std::{
	io,
	path::{Path, PathBuf},
	time::{Duration, Instant},
};

use crate::embedder::{cosine_similarity, TextEncoder};
use crate::sidecar::{iter_sidecars, ImageSidecar};
use crate::config::DEBOUNCE_TIME_MS;

/// In-memory cache of an image's embedding to avoid disk I/O during search
struct CachedImage {
	path: PathBuf,
	embedding: Vec<f32>,
}

struct AppState {
	query: String,
	results: Vec<(PathBuf, f32)>,
	index: Vec<CachedImage>,
	encoder: TextEncoder,
	status_message: String,
	is_loading: bool,
}

pub fn run_live_search(directory: &Path) -> Result<()> {
	// 1. Setup Terminal
	enable_raw_mode()?;
	let mut stdout = io::stdout();
	execute!(stdout, EnterAlternateScreen)?;
	let backend = ratatui::backend::CrosstermBackend::new(stdout);
	let mut terminal = Terminal::new(backend)?;

	// 2. Initialize State
	// We use Arc/Mutex for the encoder just in case we want to thread this later,
	// though for TUI strictly, single threaded with non-blocking poll is often fine.
	let encoder = match TextEncoder::new() {
		Ok(e) => e,
		Err(e) => {
			cleanup_terminal()?;
			return Err(e);
		}
	};

	let mut app = AppState {
		query: String::new(),
		results: Vec::new(),
		index: Vec::new(),
		encoder,
		status_message: "Loading index...".to_string(),
		is_loading: true,
	};

	// 3. Initial Draw (Loading screen)
	terminal.draw(|f| ui(f, &app))?;

	// 4. Load Index (Pre-load JSON sidecars into RAM)
	// In a real generic app, you might want to do this in a thread to keep UI responsive,
	// but for a CLI tool start-up, a synchronous load with a spinner is okay.
	let root = directory.canonicalize().unwrap_or_else(|_| directory.to_path_buf());
	let mut loaded_count = 0;

	for sidecar_path in iter_sidecars(&root) {
		if let Ok(content) = std::fs::read_to_string(&sidecar_path) {
			if let Ok(sidecar) = serde_json::from_str::<ImageSidecar>(&content) {
				app.index.push(CachedImage {
					path: PathBuf::from(sidecar.source),
					embedding: sidecar.embedding,
				});
				loaded_count += 1;

				// Optional: Update loading status every 100 images
				if loaded_count % 100 == 0 {
					app.status_message = format!("Loading index: {} images...", loaded_count);
					terminal.draw(|f| ui(f, &app))?;
				}
			}
		}
	}

	app.is_loading = false;
	app.status_message = format!("Ready. Indexed {} images.", loaded_count);

	// 5. Main Event Loop
	let mut last_input_time = Instant::now();
	let mut last_searched_query = String::new();
	// Debounce time: wait this long after typing stops to search
	let debounce_duration = Duration::from_millis(DEBOUNCE_TIME_MS);

	loop {
		terminal.draw(|f| ui(f, &app))?;

		// Poll for events (100ms tick rate)
		if event::poll(Duration::from_millis(100))? {
			if let Event::Key(key) = event::read()? {
				if key.kind == KeyEventKind::Press {
					match key.code {
						KeyCode::Esc => break,
						KeyCode::Char(c) => {
							app.query.push(c);
							last_input_time = Instant::now();
						}
						KeyCode::Backspace => {
							app.query.pop();
							last_input_time = Instant::now();
						}
						KeyCode::Enter => {
							// Force search immediately on Enter
							perform_search(&mut app);
							last_searched_query = app.query.clone();
						}
						_ => {}
					}
				}
			}
		}

		// Check Debounce logic
		let time_since_input = last_input_time.elapsed();
		if time_since_input > debounce_duration && app.query != last_searched_query {
			if !app.query.is_empty() {
				app.status_message = "Searching...".to_string();
				terminal.draw(|f| ui(f, &app))?; // Show "Searching..." immediately
				perform_search(&mut app);
			} else {
				app.results.clear();
				app.status_message = format!("Ready. Indexed {} images.", app.index.len());
			}
			last_searched_query = app.query.clone();
		}
	}

	// 6. Cleanup
	cleanup_terminal()?;
	Ok(())
}

fn perform_search(app: &mut AppState) {
	let start = Instant::now();

	// Embed query
	let query_emb = match app.encoder.embed(&app.query) {
		Ok(e) => e,
		Err(e) => {
			app.status_message = format!("Error embedding query: {}", e);
			return;
		}
	};

	// Search in-memory index
	let mut scores: Vec<(PathBuf, f32)> = app.index
		.iter()
		.map(|img| (img.path.clone(), cosine_similarity(&query_emb, &img.embedding)))
		.filter(|(_, score)| *score > 0.0) // Filter totally irrelevant
		.collect();

	// Sort desc
	scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

	app.results = scores.into_iter().take(20).collect();

	let duration = start.elapsed();
	app.status_message = format!("Found {} matches in {:.0}ms", app.results.len(), duration.as_millis());
}

fn cleanup_terminal() -> Result<()> {
	disable_raw_mode()?;
	execute!(io::stdout(), LeaveAlternateScreen)?;
	Ok(())
}

fn ui(f: &mut ratatui::Frame, app: &AppState) {
	let chunks = Layout::default()
		.direction(Direction::Vertical)
		.constraints([
			Constraint::Length(3), // Search Bar
			Constraint::Min(1),    // Results
			Constraint::Length(1), // Status Bar
		])
		.split(f.area());

	// Search Bar
	let search_text = if app.query.is_empty() {
		Line::from(vec![
			Span::styled(" 🔍 ", Style::default()),
			Span::styled("Image of...", Style::default().fg(Color::DarkGray)),
		])
	} else {
		Line::from(vec![
			Span::styled(" 🔍 ", Style::default()),
			Span::styled(&app.query, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
		])
	};

	let search_block = Paragraph::new(search_text)
		.block(Block::default().borders(Borders::ALL).title(" Search ").border_style(Style::default().fg(Color::Blue)));
	f.render_widget(search_block, chunks[0]);

	// Results List
	let items: Vec<ListItem> = app.results
		.iter()
		.enumerate()
		.map(|(i, (path, score))| {
			let filename = path.file_name().unwrap_or_default().to_string_lossy();
			let score_pct = (score * 100.0) as u32;

			// Color code score
			let score_style = if score_pct > 20 { Style::default().fg(Color::Green) }
							 else if score_pct > 10 { Style::default().fg(Color::Yellow) }
							 else { Style::default().fg(Color::DarkGray) };

			let content = Line::from(vec![
				Span::styled(format!(" {:2}. ", i + 1), Style::default().fg(Color::DarkGray)),
				Span::styled(format!("{:3}% ", score_pct), score_style),
				Span::styled(filename, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
				Span::styled(format!("  ({})", path.display()), Style::default().fg(Color::DarkGray)),
			]);

			ListItem::new(content)
		})
		.collect();

	let results_block = List::new(items)
		.block(Block::default().borders(Borders::ALL).title(" Results "));
	f.render_widget(results_block, chunks[1]);

	// Status Bar
	let status = Paragraph::new(app.status_message.as_str())
		.style(Style::default().fg(Color::DarkGray));
	f.render_widget(status, chunks[2]);
}