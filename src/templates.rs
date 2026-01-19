use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::content::Document;
use crate::generator::NavigationTree;

pub struct TemplateEngine {
	base_template: String,
}

impl TemplateEngine {
	pub fn new() -> Result<Self> {
		let base_template = include_str!("../templates/base.html").to_string();
		Ok(Self { base_template })
	}

	pub fn render_page(
		&self,
		doc: &Document,
		all_docs: &[&Document],
		navigation: &NavigationTree,
		config: &Config,
		output_path: &Path,
	) -> Result<()> {
		let html = self.render(doc, all_docs, navigation, config)?;

		// Create parent directory if needed
		if let Some(parent) = output_path.parent() {
			fs::create_dir_all(parent)?;
		}

		fs::write(output_path, html)?;
		Ok(())
	}

	fn render(
		&self,
		doc: &Document,
		_all_docs: &[&Document],
		navigation: &NavigationTree,
		config: &Config,
	) -> Result<String> {
		let title = doc
			.frontmatter
			.title
			.as_ref()
			.map(|t| t.clone())
			.unwrap_or_else(|| "Untitled".to_string());

		let site_title = &config.site.title;
		let page_title = format!("{} - {}", title, site_title);

		// Render sidebar
		let sidebar_html = self.render_sidebar(navigation, &doc.relative_path);

		// Render breadcrumbs
		let breadcrumbs_html = if config.navigation.breadcrumbs {
			self.render_breadcrumbs(&doc.relative_path)
		} else {
			String::new()
		};

		// Render backlinks
		let backlinks_html = if !doc.backlinks.is_empty() {
			self.render_backlinks(&doc.backlinks)
		} else {
			String::new()
		};

		// Render version selector
		let version_selector = self.render_version_selector(&config.site.versions, &doc.version);

		// Replace template variables
		let html = self
			.base_template
			.replace("{{SITE_TITLE}}", site_title)
			.replace("{{PAGE_TITLE}}", &page_title)
			.replace("{{TITLE}}", &title)
			.replace("{{CONTENT}}", &doc.html_content)
			.replace("{{SIDEBAR}}", &sidebar_html)
			.replace("{{BREADCRUMBS}}", &breadcrumbs_html)
			.replace("{{BACKLINKS}}", &backlinks_html)
			.replace("{{VERSION_SELECTOR}}", &version_selector)
			.replace(
				"{{DEFAULT_THEME}}",
				config.theme.default_theme.as_deref().unwrap_or("light"),
			)
			.replace(
				"{{SEARCH_ENABLED}}",
				if config.search.enabled {
					"true"
				} else {
					"false"
				},
			);

		Ok(html)
	}

	fn render_sidebar(&self, navigation: &NavigationTree, current_path: &Path) -> String {
		let mut html = String::from("<nav class=\"sidebar\">\n<ul>\n");

		for item in &navigation.items {
			html.push_str(&self.render_nav_item(item, current_path, 0));
		}

		html.push_str("</ul>\n</nav>");
		html
	}

	fn render_nav_item(
		&self,
		item: &crate::generator::NavigationItem,
		current_path: &Path,
		depth: usize,
	) -> String {
		let indent = "  ".repeat(depth);
		let is_active =
			!item.path.as_os_str().is_empty() && item.path.file_stem() == current_path.file_stem();
		let active_class = if is_active { " class=\"active\"" } else { "" };

		let mut html = format!("{}<li{}>\n", indent, active_class);

		if !item.path.as_os_str().is_empty() {
			let mut href = item.path.to_string_lossy().replace('\\', "/");
			// Change .md to .html
			if href.ends_with(".md") {
				href = href.replace(".md", ".html");
			}
			// Add version prefix if needed
			if let Some(version) = &item.version {
				// Only prepend if the path doesn't already start with the version
				if !href.starts_with(version) {
					href = format!("{}/{}", version, href);
				}
			}
			href = format!("/{}", href);
			html.push_str(&format!(
				"{}<a href=\"{}\">{}</a>\n",
				"  ".repeat(depth + 1),
				href,
				item.title
			));
		} else {
			html.push_str(&format!(
				"{}<span>{}</span>\n",
				"  ".repeat(depth + 1),
				item.title
			));
		}

		if !item.children.is_empty() {
			html.push_str(&format!("{}<ul>\n", "  ".repeat(depth + 1)));
			for child in &item.children {
				html.push_str(&self.render_nav_item(child, current_path, depth + 1));
			}
			html.push_str(&format!("{}</ul>\n", "  ".repeat(depth + 1)));
		}

		html.push_str(&format!("{}</li>\n", indent));
		html
	}

	fn render_breadcrumbs(&self, path: &Path) -> String {
		let mut html = String::from("<nav class=\"breadcrumbs\">\n");
		html.push_str("<a href=\"/\">Home</a>");

		let components: Vec<_> = path.components().collect();
		let mut current_path = PathBuf::new();

		for component in components {
			current_path.push(component);
			let name = component.as_os_str().to_string_lossy();
			let mut href = current_path.to_string_lossy().replace('\\', "/");
			// Change .md to .html
			if href.ends_with(".md") {
				href = href.replace(".md", ".html");
			}
			href = format!("/{}", href);
			html.push_str(&format!(
				" / <a href=\"{}\">{}</a>",
				href,
				name.replace(".html", "")
			));
		}

		html.push_str("\n</nav>");
		html
	}

	fn render_backlinks(&self, backlinks: &[String]) -> String {
		let mut html =
			String::from("<div class=\"backlinks\">\n<h3>Pages that link here</h3>\n<ul>\n");

		for link in backlinks {
			html.push_str(&format!(
				"<li><a href=\"#{}\">{}</a></li>\n",
				link.to_lowercase().replace(' ', "-"),
				link
			));
		}

		html.push_str("</ul>\n</div>");
		html
	}

	fn render_version_selector(
		&self,
		versions: &[String],
		current_version: &Option<String>,
	) -> String {
		if versions.len() <= 1 {
			return String::new();
		}

		let mut html = String::from(
			"<select id=\"version-selector\" onchange=\"switchVersion(this.value)\">\n",
		);

		for version in versions {
			let selected = if current_version
				.as_ref()
				.map(|v| v == version)
				.unwrap_or(false)
			{
				" selected"
			} else {
				""
			};
			html.push_str(&format!(
				"<option value=\"{}\"{}>{}</option>\n",
				version, selected, version
			));
		}

		html.push_str("</select>");
		html
	}
}
