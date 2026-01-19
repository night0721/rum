use anyhow::Result;
use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;

use crate::config::Config;
use crate::generator::Generator;
use crate::server::DevServer;

#[derive(Parser)]
#[command(name = "rum")]
#[command(about = "A next-gen static documentation/wiki generator")]
#[command(version)]
pub struct Cli {
	#[command(subcommand)]
	pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
	/// Build static site
	Build {
		/// Source directory (default: docs/)
		#[arg(short, long, default_value = "docs")]
		source: PathBuf,

		/// Output directory (default: dist/)
		#[arg(short, long, default_value = "dist")]
		output: PathBuf,

		/// Export formats (html, pdf, man)
		#[arg(short, long, default_value = "html")]
		format: String,

		/// Configuration file
		#[arg(short, long)]
		config: Option<PathBuf>,
	},

	/// Start development server
	Dev {
		/// Source directory (default: docs/)
		#[arg(short, long, default_value = "docs")]
		source: PathBuf,

		/// Port to serve on
		#[arg(short, long, default_value_t = 3000)]
		port: u16,

		/// Configuration file
		#[arg(short, long)]
		config: Option<PathBuf>,
	},

	/// Initialize a new Rum project
	Init {
		/// Directory to initialize
		#[arg(default_value = ".")]
		dir: PathBuf,
	},
}

impl Cli {
	pub async fn run(self) -> Result<()> {
		match self.command {
			Commands::Build {
				source,
				output,
				format,
				config,
			} => {
				let output_clone = output.clone();
				let generator = Generator::new(source, output, config)?;
				generator.build(&format).await?;
				println!("Build complete. Output: {}", output_clone.display());
			}
			Commands::Dev {
				source,
				port,
				config,
			} => {
				let server = DevServer::new(source, port, config)?;
				server.serve().await?;
			}
			Commands::Init { dir } => {
				// Create docs directory
				let docs_dir = dir.join("docs");
				fs::create_dir_all(&docs_dir)?;
				fs::create_dir_all(docs_dir.join("latest"))?;

				// Create example docs
				let example_content = r#"---
title: Welcome to Rum
tags: [getting-started]
---

# Welcome to Rum
This is your first documentation page. Edit this file to get started!

## Getting Started
1. Edit this file
2. Configure the site in \`rum.toml\`
3. Add more `.md` files to the `docs/` directory
4. Run `rum dev` to preview
5. Run `rum build` to generate static site

## Shortcodes
Use shortcodes for special content:
{{note}}
This is a note block!
{{/note}}
"#;
				fs::write(docs_dir.join("index.md"), example_content)?;

				let latest_content = r#"---
title: Latest Version
version: latest
tags: [docs]
---

# Documentation for Latest Version

Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.
"#;
				fs::write(docs_dir.join("latest").join("index.md"), latest_content)?;
				let config = Config::default();
				config.save(&dir.join("rum.toml"))?;

				println!("Initialized project in {}", dir.display());
			}
		}
		Ok(())
	}
}
