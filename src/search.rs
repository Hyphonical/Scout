// Search - Find images by matching keywords against tags
//
// Supports two modes:
// 1. Keyword matching with advanced syntax (-word, (a~b), word~, wo*rd)
// 2. Semantic search using text embeddings (when available)

use std::fs;
use std::path::{Path, PathBuf};

use crate::config::SIDECAR_DIR;
use crate::embedder::{cosine_similarity, TextEmbedder};
use crate::sidecar::{iter_sidecars, ImageSidecar};

pub struct SearchResult {
	pub image_path: PathBuf,
	pub score: f32,
	pub matched_tags: Vec<MatchedTag>,
	pub semantic: bool,
}

pub struct MatchedTag {
	pub query_term: String,
	pub tag_name: String,
}

#[derive(Debug, Clone)]
enum Term {
	Include(String),
	Exclude(String),
	Fuzzy(String),
	Wildcard(String),
	Or(Vec<String>),
}

/// Parses query string into structured terms.
fn parse_query(query: &str) -> Vec<Term> {
	let mut terms = Vec::new();
	let query = query.to_lowercase();
	let mut chars = query.chars().peekable();
	
	while let Some(c) = chars.next() {
		match c {
			' ' | '\t' => continue,
			'-' => {
				let word: String = chars.by_ref().take_while(|&c| c != ' ').collect();
				if !word.is_empty() {
					terms.push(Term::Exclude(word));
				}
			}
			'(' => {
				// Parse OR group: (a ~ b ~ c)
				let group: String = chars.by_ref().take_while(|&c| c != ')').collect();
				let parts: Vec<String> = group.split('~')
					.map(|s| s.trim().to_string())
					.filter(|s| !s.is_empty())
					.collect();
				if parts.len() > 1 {
					terms.push(Term::Or(parts));
				} else if let Some(p) = parts.into_iter().next() {
					terms.push(Term::Include(p));
				}
			}
			_ => {
				let mut word = String::from(c);
				for ch in chars.by_ref() {
					if ch == ' ' { break; }
					word.push(ch);
				}
				
				if word.ends_with('~') {
					word.pop();
					if !word.is_empty() {
						terms.push(Term::Fuzzy(word));
					}
				} else if word.contains('*') {
					terms.push(Term::Wildcard(word));
				} else if !word.is_empty() {
					terms.push(Term::Include(word));
				}
			}
		}
	}
	
	terms
}

/// Searches all sidecar files for images matching the query.
/// Uses semantic search if embeddings are available, otherwise keyword matching.
pub fn search_images(root: &Path, query: &str, min_score: f32, semantic: bool) -> Vec<SearchResult> {
	let scout_dir = root.join(SIDECAR_DIR);
	if !scout_dir.exists() {
		return Vec::new();
	}

	// Try semantic search if requested and embedder is available
	if semantic {
		if let Some(results) = search_semantic(root, query, min_score) {
			return results;
		}
	}

	// Fall back to keyword search
	search_keywords(root, query, min_score)
}

/// Semantic search using text embeddings.
fn search_semantic(root: &Path, query: &str, min_score: f32) -> Option<Vec<SearchResult>> {
	let embedder = TextEmbedder::new().ok()?;
	let query_embedding = embedder.embed_text(query).ok()?;

	let mut results = Vec::new();

	for path in iter_sidecars(root) {
		let Ok(content) = fs::read_to_string(&path) else { continue };
		let Ok(sidecar) = serde_json::from_str::<ImageSidecar>(&content) else { continue };

		// Skip if no embedding stored
		let Some(embedding) = sidecar.embedding.as_ref() else { continue };
		
		let similarity = cosine_similarity(&query_embedding, embedding);
		
		// Convert similarity (-1 to 1) to score (0 to 1)
		let score = (similarity + 1.0) / 2.0;

		if score >= min_score {
			// Find top matching tags for display
			let top_tags: Vec<MatchedTag> = sidecar.tags.iter()
				.take(3)
				.map(|t| MatchedTag {
					query_term: query.to_string(),
					tag_name: t.name.clone(),
				})
				.collect();

			results.push(SearchResult {
				image_path: PathBuf::from(&sidecar.source_image),
				score,
				matched_tags: top_tags,
				semantic: true,
			});
		}
	}

	if results.is_empty() {
		return None;
	}

	results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
	Some(results)
}

/// Keyword-based search with advanced syntax.
fn search_keywords(root: &Path, query: &str, min_score: f32) -> Vec<SearchResult> {
	let terms = parse_query(query);
	if terms.is_empty() {
		return Vec::new();
	}

	let mut results = Vec::new();

	for path in iter_sidecars(root) {
		if let Some(result) = score_sidecar(&path, &terms) {
			if result.score >= min_score {
				results.push(result);
			}
		}
	}

	results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
	results
}

fn score_sidecar(sidecar_path: &Path, terms: &[Term]) -> Option<SearchResult> {
	let content = fs::read_to_string(sidecar_path).ok()?;
	let sidecar: ImageSidecar = serde_json::from_str(&content).ok()?;

	let mut matched_tags = Vec::new();
	let mut total_score = 0.0;
	let mut include_count = 0;

	for term in terms {
		match term {
			Term::Exclude(word) => {
				// If any tag matches the exclusion, reject this image
				for tag in &sidecar.tags {
					if tag.name.to_lowercase().contains(word) {
						return None;
					}
				}
			}
			Term::Include(word) => {
				include_count += 1;
				if let Some((tag_name, quality)) = find_best_match(&sidecar.tags, word) {
					matched_tags.push(MatchedTag {
						query_term: word.clone(),
						tag_name,
					});
					total_score += quality;
				}
			}
			Term::Fuzzy(word) => {
				include_count += 1;
				if let Some((tag_name, quality)) = find_fuzzy_match(&sidecar.tags, word) {
					matched_tags.push(MatchedTag {
						query_term: format!("{}~", word),
						tag_name,
					});
					total_score += quality;
				}
			}
			Term::Wildcard(pattern) => {
				include_count += 1;
				if let Some((tag_name, quality)) = find_wildcard_match(&sidecar.tags, pattern) {
					matched_tags.push(MatchedTag {
						query_term: pattern.clone(),
						tag_name,
					});
					total_score += quality;
				}
			}
			Term::Or(words) => {
				include_count += 1;
				// Find best match among any of the OR terms
				let mut best: Option<(String, String, f32)> = None;
				for word in words {
					if let Some((tag_name, quality)) = find_best_match(&sidecar.tags, word) {
						if best.as_ref().map(|(_, _, q)| quality > *q).unwrap_or(true) {
							best = Some((word.clone(), tag_name, quality));
						}
					}
				}
				if let Some((query_term, tag_name, quality)) = best {
					matched_tags.push(MatchedTag { query_term, tag_name });
					total_score += quality;
				}
			}
		}
	}

	if matched_tags.is_empty() || include_count == 0 {
		return None;
	}

	let score = total_score / include_count as f32;
	let image_path = PathBuf::from(&sidecar.source_image);

	Some(SearchResult { image_path, score, matched_tags, semantic: false })
}

fn find_best_match(tags: &[crate::sidecar::TagEntry], term: &str) -> Option<(String, f32)> {
	let mut best: Option<(&str, f32)> = None;
	
	for tag in tags {
		let tag_lower = tag.name.to_lowercase();
		let quality = match_quality(&tag_lower, term);
		
		if quality > 0.0 && best.map(|(_, q)| quality > q).unwrap_or(true) {
			best = Some((&tag.name, quality));
		}
	}
	
	best.map(|(n, q)| (n.to_string(), q))
}

fn find_fuzzy_match(tags: &[crate::sidecar::TagEntry], term: &str) -> Option<(String, f32)> {
	let mut best: Option<(&str, f32)> = None;
	
	for tag in tags {
		let tag_lower = tag.name.to_lowercase();
		
		// Check each part of underscore-separated tags
		for part in tag_lower.split('_') {
			let dist = levenshtein(part, term);
			let max_len = part.len().max(term.len());
			
			if max_len > 0 && dist <= 2 {
				let quality = 1.0 - (dist as f32 / max_len as f32);
				if best.map(|(_, q)| quality > q).unwrap_or(true) {
					best = Some((&tag.name, quality));
				}
			}
		}
	}
	
	best.map(|(n, q)| (n.to_string(), q))
}

fn find_wildcard_match(tags: &[crate::sidecar::TagEntry], pattern: &str) -> Option<(String, f32)> {
	if pattern == "*" {
		return tags.first().map(|t| (t.name.clone(), 0.5));
	}

	let parts: Vec<&str> = pattern.split('*').collect();
	
	for tag in tags {
		let tag_lower = tag.name.to_lowercase();
		
		let matches = match parts.as_slice() {
			[prefix, suffix] if !prefix.is_empty() && !suffix.is_empty() => {
				tag_lower.starts_with(prefix) && tag_lower.ends_with(suffix)
			}
			[prefix, _] if !prefix.is_empty() => tag_lower.starts_with(prefix),
			[_, suffix] if !suffix.is_empty() => tag_lower.ends_with(suffix),
			_ => false,
		};
		
		if matches {
			return Some((tag.name.clone(), 0.8));
		}
	}
	
	None
}

fn match_quality(tag: &str, term: &str) -> f32 {
	if tag == term { return 1.0; }
	if tag.split('_').any(|p| p == term) { return 0.9; }
	if tag.starts_with(term) { return 0.8; }
	if tag.contains(term) { return 0.6; }
	if tag.split('_').any(|p| p.starts_with(term)) { return 0.5; }
	0.0
}

fn levenshtein(a: &str, b: &str) -> usize {
	let a: Vec<char> = a.chars().collect();
	let b: Vec<char> = b.chars().collect();
	let (m, n) = (a.len(), b.len());
	
	if m == 0 { return n; }
	if n == 0 { return m; }
	
	let mut prev: Vec<usize> = (0..=n).collect();
	let mut curr = vec![0; n + 1];
	
	for i in 1..=m {
		curr[0] = i;
		for j in 1..=n {
			let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
			curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
		}
		std::mem::swap(&mut prev, &mut curr);
	}
	
	prev[n]
}
