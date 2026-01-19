use anyhow::Result;
use std::path::Path;

use crate::config::Config;
use crate::content::Document;

pub struct Exporter {
	output_dir: std::path::PathBuf,
}

impl Exporter {
	pub fn new(output_dir: &Path) -> Self {
		Self {
			output_dir: output_dir.to_path_buf(),
		}
	}

	pub async fn export_pdfs(&self, _documents: &[Document], _config: &Config) -> Result<()> {
		// PDF export placeholder
		println!("PDF export not yet fully implemented");
		Ok(())
	}

	pub async fn export_man_pages(&self, _documents: &[Document], _config: &Config) -> Result<()> {
		// Man page(roff) export placeholder
		println!("Man page export not yet fully implemented");
		Ok(())
	}
}
