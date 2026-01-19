use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::config::Config;
use crate::content::{ContentProcessor, Document};
use crate::export::Exporter;
use crate::templates::TemplateEngine;

pub struct Generator {
	source_dir: PathBuf,
	output_dir: PathBuf,
	config: Config,
	processor: ContentProcessor,
	template_engine: TemplateEngine,
}

impl Generator {
	pub fn new(
		source_dir: PathBuf,
		output_dir: PathBuf,
		config_path: Option<PathBuf>,
	) -> Result<Self> {
		let config = Config::load(config_path.as_deref())?;
		let processor = ContentProcessor::new();
		let template_engine = TemplateEngine::new()?;

		Ok(Self {
			source_dir,
			output_dir,
			config,
			processor,
			template_engine,
		})
	}

	pub async fn build(&self, formats: &str) -> Result<()> {
		// Clean output directory
		if self.output_dir.exists() {
			fs::remove_dir_all(&self.output_dir)?;
		}
		fs::create_dir_all(&self.output_dir)?;

		// Collect all documents
		let documents = self.collect_documents()?;

		// Process backlinks
		let documents = self.process_backlinks(documents);

		// Build navigation structure
		let navigation = self.build_navigation(&documents);

		// Generate search index
		let search_index = self.generate_search_index(&documents);

		// Generate HTML
		if formats.contains("html") {
			self.generate_html(&documents, &navigation, &search_index)
				.await?;
		}

		// Generate PDFs
		if formats.contains("pdf") {
			let exporter = Exporter::new(&self.output_dir);
			exporter.export_pdfs(&documents, &self.config).await?;
		}

		// Generate man pages
		if formats.contains("man") {
			let exporter = Exporter::new(&self.output_dir);
			exporter.export_man_pages(&documents, &self.config).await?;
		}

		Ok(())
	}

	fn collect_documents(&self) -> Result<Vec<Document>> {
		let mut documents = Vec::new();

		for entry in WalkDir::new(&self.source_dir)
			.follow_links(true)
			.into_iter()
			.filter_map(|e| e.ok())
		{
			let path = entry.path();

			if path.is_file() {
				let ext = path.extension().and_then(|s| s.to_str());
				if matches!(ext, Some("md" | "rst" | "txt" | "adoc")) {
					match ContentProcessor::parse_document(path, &self.source_dir) {
						Ok(doc) => documents.push(doc),
						Err(e) => eprintln!("Warning: Failed to parse {}: {}", path.display(), e),
					}
				}
			}
		}

		// Sort by order if specified
		documents.sort_by(|a, b| {
			let a_order = a.frontmatter.order.unwrap_or(999);
			let b_order = b.frontmatter.order.unwrap_or(999);
			a_order.cmp(&b_order)
		});

		Ok(documents)
	}

	fn process_backlinks(&self, mut documents: Vec<Document>) -> Vec<Document> {
		// Create a map of document titles/paths to their indices
		let mut doc_map: HashMap<String, usize> = HashMap::new();

		for (idx, doc) in documents.iter().enumerate() {
			if let Some(title) = &doc.frontmatter.title {
				doc_map.insert(title.to_lowercase(), idx);
			}
			// Also index by path
			let path_key = doc.relative_path.to_string_lossy().to_lowercase();
			doc_map.insert(path_key, idx);
		}

		// Collect backlink updates
		let mut backlink_updates: Vec<(usize, String)> = Vec::new();

		// Process backlinks
		for doc in &documents {
			for link in &doc.links {
				let link_lower = link.to_lowercase();
				if let Some(&target_idx) = doc_map.get(&link_lower) {
					let doc_title = doc
						.frontmatter
						.title
						.as_ref()
						.map(|t| t.clone())
						.unwrap_or_else(|| doc.relative_path.to_string_lossy().to_string());

					backlink_updates.push((target_idx, doc_title));
				}
			}
		}

		// Apply backlink updates
		for (idx, title) in backlink_updates {
			documents[idx].backlinks.push(title);
		}

		documents
	}

	fn build_navigation(&self, documents: &[Document]) -> NavigationTree {
		let mut tree = NavigationTree::new();

		for doc in documents {
			let path = &doc.relative_path;
			let title = doc
				.frontmatter
				.title
				.as_ref()
				.map(|t| t.clone())
				.unwrap_or_else(|| {
					path.file_stem()
						.and_then(|s| s.to_str())
						.unwrap_or("Untitled")
						.to_string()
				});

			tree.add_path(path, title, doc.version.clone());
		}

		tree
	}

	fn generate_search_index(&self, documents: &[Document]) -> String {
		use serde_json::json;

		let search_docs: Vec<_> = documents
            .iter()
            .map(|doc| {
                json!({
                    "title": doc.frontmatter.title.as_ref().unwrap_or(&doc.relative_path.to_string_lossy().to_string()),
                    "content": doc.content,
                    "path": doc.relative_path.to_string_lossy(),
                    "version": doc.version,
                })
            })
            .collect();

		serde_json::to_string(&search_docs).unwrap_or_default()
	}

	async fn generate_html(
		&self,
		documents: &[Document],
		navigation: &NavigationTree,
		search_index: &str,
	) -> Result<()> {
		// Create output directories
		fs::create_dir_all(self.output_dir.join("assets"))?;
		fs::create_dir_all(self.output_dir.join("assets/css"))?;
		fs::create_dir_all(self.output_dir.join("assets/js"))?;

		// Copy static assets
		self.copy_assets()?;

		// Write search index
		fs::write(
			self.output_dir.join("assets/search-index.json"),
			search_index,
		)?;

		// Group documents by version
		let mut docs_by_version: HashMap<Option<String>, Vec<&Document>> = HashMap::new();
		for doc in documents {
			docs_by_version
				.entry(doc.version.clone())
				.or_insert_with(Vec::new)
				.push(doc);
		}

		// Generate pages for each version
		for (version, docs) in &docs_by_version {
			let version_path = if let Some(v) = version {
				self.output_dir.join(v)
			} else {
				self.output_dir.clone()
			};
			fs::create_dir_all(&version_path)?;

            /*
			// Generate index page - use a doc named index.md or first doc
			let index_doc = docs
				.iter()
				.find(|d| d.relative_path.file_stem().and_then(|s| s.to_str()) == Some("index"))
				.or_else(|| docs.first())
				.ok_or_else(|| anyhow::anyhow!("No documents found"))?;

			self.template_engine.render_page(
				index_doc,
				docs,
				navigation,
				&self.config,
				&version_path.join("index.html"),
			)?;*/

			// Generate individual pages
			for doc in docs {
                /*
				// Skip index.md as we already generated it
				if doc.relative_path.file_stem().and_then(|s| s.to_str()) == Some("index") {
					continue;
				}
                */

                let stripped_path = if let Some(v) = version {
                    doc.relative_path.strip_prefix(v).unwrap_or(&doc.relative_path)
                } else {
                    &doc.relative_path
                };

/* 				let html_path = version_path.join(doc.relative_path.with_extension("html")); */
                let html_path = version_path.join(stripped_path.with_extension("html"));

				// Create parent directories
				if let Some(parent) = html_path.parent() {
					fs::create_dir_all(parent)?;
				}

				self.template_engine.render_page(
					doc,
					docs,
					navigation,
					&self.config,
					&html_path,
				)?;
			}
		}

		Ok(())
	}

	fn copy_assets(&self) -> Result<()> {
		// Copy CSS
		let css = include_str!("../templates/assets/style.css");
		fs::write(self.output_dir.join("assets/css/style.css"), css)?;

		// Copy JS
		let js = include_str!("../templates/assets/app.js");
		fs::write(self.output_dir.join("assets/js/app.js"), js)?;

		Ok(())
	}
}

#[derive(Debug, Clone)]
pub struct NavigationTree {
	pub items: Vec<NavigationItem>,
}

#[derive(Debug, Clone)]
pub struct NavigationItem {
	pub title: String,
	pub path: PathBuf,
	pub children: Vec<NavigationItem>,
	pub version: Option<String>,
}

impl NavigationTree {
	pub fn new() -> Self {
		Self { items: Vec::new() }
	}

	pub fn add_path(&mut self, path: &Path, title: String, version: Option<String>) {
		let components: Vec<_> = path.components().collect();
		let mut current = &mut self.items;

		for (idx, component) in components.iter().enumerate() {
			let name = component.as_os_str().to_string_lossy().to_string();
			let is_file = idx == components.len() - 1;

			if is_file {
				current.push(NavigationItem {
					title: title.clone(),
					path: path.to_path_buf(),
					children: Vec::new(),
					version: version.clone(),
				});
			} else {
				// Find or create directory node
				let existing_idx = current
					.iter()
					.position(|item| item.title == name && item.path.as_os_str().is_empty());

				if let Some(existing_idx) = existing_idx {
					current = &mut current[existing_idx].children;
				} else {
					let new_item = NavigationItem {
						title: name.clone(),
						path: PathBuf::new(),
						children: Vec::new(),
						version: None,
					};
					current.push(new_item);
					let last_idx = current.len() - 1;
					current = &mut current[last_idx].children;
				}
			}
		}
	}
}

impl Default for NavigationTree {
	fn default() -> Self {
		Self::new()
	}
}
