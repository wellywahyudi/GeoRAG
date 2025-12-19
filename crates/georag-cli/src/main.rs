//! GeoRAG CLI - Command-line interface
//!
//! This is the main CLI adapter for the GeoRAG system.

mod cli;
mod commands;
mod config_loader;
mod dry_run;
mod output;
mod output_types;

use anyhow::Result;
use clap::Parser;
use cli::Cli;

fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Parse CLI arguments
    let cli = Cli::parse();

    // Execute the command
    commands::execute(cli)?;

    Ok(())
}
