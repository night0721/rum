mod cli;
mod config;
mod content;
mod export;
mod generator;
mod server;
mod templates;

use anyhow::Result;
use clap::Parser;

use crate::cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
	let cli = Cli::parse();
	cli.run().await
}
