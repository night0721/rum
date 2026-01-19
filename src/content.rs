use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
	pub frontmatter: Frontmatter,
	pub content: String,
	pub html_content: String,
	pub path: PathBuf,
	pub relative_path: PathBuf,
	pub version: Option<String>,
	pub backlinks: Vec<String>,
	pub links: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Frontmatter {
	pub title: Option<String>,
	pub version: Option<String>,
	pub tags: Option<Vec<String>>,
	pub author: Option<String>,
	pub description: Option<String>,
	pub order: Option<u32>,
	#[serde(flatten)]
	pub extra: HashMap<String, serde_yaml::Value>,
}

#[derive(Debug, Clone)]
pub struct ContentProcessor {
	wiki_link_regex: Regex,
	shortcode_regex: Regex,
}

impl ContentProcessor {
	pub fn new() -> Self {
		Self {
			wiki_link_regex: Regex::new(r"\[\[([^\]]+)\]\]").unwrap(),
			shortcode_regex: Regex::new(r"\{\{([^}]+)\}\}").unwrap(),
		}
	}

	pub fn parse_document(path: &Path, base_path: &Path) -> Result<Document> {
		let content = fs::read_to_string(path)
			.with_context(|| format!("Failed to read file: {}", path.display()))?;

		let (frontmatter, markdown_content) = Self::extract_frontmatter(&content)?;

		// Detect version from path
		let version = Self::extract_version(path, base_path);

		// Process wiki links and shortcodes
		let processed_content = Self::process_content(&markdown_content);

		// Convert markdown to HTML
		let html_content = Self::markdown_to_html(&processed_content);

		// Extract links
		let links = Self::extract_links(&processed_content);

		let relative_path = path.strip_prefix(base_path).unwrap_or(path).to_path_buf();

		Ok(Document {
			frontmatter,
			content: processed_content,
			html_content,
			path: path.to_path_buf(),
			relative_path,
			version,
			backlinks: vec![],
			links,
		})
	}

	fn extract_frontmatter(content: &str) -> Result<(Frontmatter, String)> {
		// Try YAML frontmatter
		if content.starts_with("---\n") {
			if let Some(end) = content[4..].find("\n---\n") {
				let frontmatter_str = &content[4..end + 4];
				let markdown = &content[end + 9..];

				let frontmatter: Frontmatter =
					serde_yaml::from_str(frontmatter_str).unwrap_or_default();

				return Ok((frontmatter, markdown.to_string()));
			}
		}

		// Try JSON frontmatter
		if content.starts_with("```json\n") {
			if let Some(end) = content.find("\n```\n") {
				let frontmatter_str = &content[8..end];
				let markdown = &content[end + 6..];

				if let Ok(frontmatter) = serde_json::from_str::<Frontmatter>(frontmatter_str) {
					return Ok((frontmatter, markdown.to_string()));
				}
			}
		}

		// Try TOML frontmatter
		if content.starts_with("+++\n") {
			if let Some(end) = content[4..].find("\n+++\n") {
				let frontmatter_str = &content[4..end + 4];
				let markdown = &content[end + 9..];

				if let Ok(frontmatter) = toml::from_str::<Frontmatter>(frontmatter_str) {
					return Ok((frontmatter, markdown.to_string()));
				}
			}
		}

		Ok((Frontmatter::default(), content.to_string()))
	}

	fn extract_version(path: &Path, base_path: &Path) -> Option<String> {
		let relative = path.strip_prefix(base_path).ok()?;
		let components: Vec<_> = relative.components().collect();

		if components.len() > 1 {
			if let Some(component) = components.first() {
				let version_str = component.as_os_str().to_string_lossy();
				if version_str.starts_with('v') || version_str == "latest" {
					return Some(version_str.to_string());
				}
			}
		}

		None
	}

	fn process_content(content: &str) -> String {
		let mut processed = content.to_string();

		// Process wiki links - convert [[Page Name]] to Markdown links
		let wiki_link_regex = Regex::new(r"\[\[([^\]]+)\]\]").unwrap();
		processed = wiki_link_regex
			.replace_all(&processed, |caps: &regex::Captures| {
				let page_name = caps.get(1).unwrap().as_str();
				// Convert to slug for URL
				let slug = page_name.to_lowercase().replace(' ', "-");
				format!("[{}]({}.html)", page_name, slug)
			})
			.to_string();

		// Process shortcodes (basic implementation)
		// {{note}}...{{/note}}
		// {{youtube:ID}}
		// etc.

		processed
	}

	fn markdown_to_html(markdown: &str) -> String {
		use pulldown_cmark::{html, Options, Parser};

		let mut options = Options::empty();
		options.insert(Options::ENABLE_STRIKETHROUGH);
		options.insert(Options::ENABLE_TABLES);
		options.insert(Options::ENABLE_TASKLISTS);
		options.insert(Options::ENABLE_SMART_PUNCTUATION);

		let parser = Parser::new_ext(markdown, options);
		let mut html_output = String::new();
		html::push_html(&mut html_output, parser);

		html_output
	}

	fn extract_links(content: &str) -> Vec<String> {
		let mut links = Vec::new();

		// Extract wiki links [[Page Name]]
		let wiki_link_regex = Regex::new(r"\[\[([^\]]+)\]\]").unwrap();
		for cap in wiki_link_regex.captures_iter(content) {
			if let Some(link) = cap.get(1) {
				links.push(link.as_str().to_string());
			}
		}

		// Extract Markdown links
		let md_link_regex = Regex::new(r"\[([^\]]+)\]\(([^)]+)\)").unwrap();
		for cap in md_link_regex.captures_iter(content) {
			if let Some(link) = cap.get(2) {
				let link_str = link.as_str();
				if !link_str.starts_with("http") {
					links.push(link_str.to_string());
				}
			}
		}

		links
	}
}

impl Default for ContentProcessor {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_extract_frontmatter() {
		let content = r#"---
title: Test Page
version: 0.1
tags: [test, example]
author: night0721
description: Example
---
# Content here
"#;
		let (fm, md) = ContentProcessor::extract_frontmatter(content).unwrap();
		assert_eq!(fm.title, Some("Test Page".to_string()));
		assert_eq!(fm.version, Some("0.1".to_string()));
		assert_eq!(fm.author, Some("night0721".to_string()));
		assert_eq!(fm.description, Some("Example".to_string()));
		assert!(md.contains("Content here"));
	}
}
