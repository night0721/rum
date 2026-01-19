use anyhow::Result;
use axum::{
	extract::Path as AxumPath,
	http::StatusCode,
	response::{Html, IntoResponse},
	routing::get,
	Router,
};
use notify::{RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceBuilder;
use tower_http::services::ServeDir;

use crate::generator::Generator;

pub struct DevServer {
	source_dir: PathBuf,
	port: u16,
	config: Option<PathBuf>,
	generator: Arc<RwLock<Option<Generator>>>,
}

impl DevServer {
	pub fn new(source_dir: PathBuf, port: u16, config: Option<PathBuf>) -> Result<Self> {
		let generator = Arc::new(RwLock::new(None));

		Ok(Self {
			source_dir,
			port,
			config,
			generator,
		})
	}

	pub async fn serve(&self) -> Result<()> {
		// Create temp output directory
		let output_dir = std::env::temp_dir().join("rum");

		// Initial build
		let generator = Generator::new(
			self.source_dir.clone(),
			output_dir.clone(),
			self.config.clone(),
		)?;

		let gen = generator;
		gen.build("html").await?;
		*self.generator.write().await = Some(gen);

		// Get a handle to the current tokio runtime to use inside the watcher thread
		let rt = tokio::runtime::Handle::current();

		let mut watcher = notify::recommended_watcher({
			let _source_dir = self.source_dir.clone();
			let generator = Arc::clone(&self.generator);
			let _output_dir = output_dir.clone();
			let rt = rt.clone();

			move |event: Result<notify::Event, notify::Error>| {
				if let Ok(event) = event {
					if event.kind.is_modify() || event.kind.is_create() || event.kind.is_remove() {
						let generator = Arc::clone(&generator);

						rt.spawn(async move {
							if let Some(gen) = generator.write().await.take() {
								let g = gen;
								if let Err(e) = g.build("html").await {
									eprintln!("Rebuild error: {}", e);
								}
								*generator.write().await = Some(g);
							}
						});
					}
				}
			}
		})?;

		watcher.watch(&self.source_dir, RecursiveMode::Recursive)?;

		// Setup HTTP server
		let app = Router::new()
			.route("/", get(serve_index))
			.route("/{*path}", get(serve_page))
			.nest_service("/assets", ServeDir::new(output_dir.join("assets")))
			.layer(ServiceBuilder::new());

		let addr = format!("0.0.0.0:{}", self.port);
		let listener = tokio::net::TcpListener::bind(&addr).await?;

		println!(
			"Development server running at http://localhost:{}",
			self.port
		);
		println!("Watching for changes...");

		axum::serve(listener, app).await?;

		Ok(())
	}

	async fn rebuild(&self) -> Result<()> {
		if let Some(ref mut gen) = *self.generator.write().await {
			gen.build("html").await?;
		}
		Ok(())
	}
}

async fn serve_index() -> impl IntoResponse {
	let output_dir = std::env::temp_dir().join("rum");
	let index_path = output_dir.join("index.html");

	if index_path.exists() {
		match tokio::fs::read_to_string(&index_path).await {
			Ok(content) => Html(content).into_response(),
			Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Failed to read file").into_response(),
		}
	} else {
		(StatusCode::NOT_FOUND, "Not found").into_response()
	}
}

async fn serve_page(AxumPath(path): AxumPath<String>) -> impl IntoResponse {
	let output_dir = std::env::temp_dir().join("rum");
	let page_path = output_dir.join(&path);

	if page_path.exists() && page_path.is_file() {
		match tokio::fs::read_to_string(&page_path).await {
			Ok(content) => Html(content).into_response(),
			Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Failed to read file").into_response(),
		}
	} else {
		(StatusCode::NOT_FOUND, "Not found").into_response()
	}
}
