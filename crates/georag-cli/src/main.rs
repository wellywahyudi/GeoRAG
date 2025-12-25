//! GeoRAG CLI - Command-line interface
//!
//! This is the main CLI adapter for the GeoRAG system.

mod cli;
mod commands;
mod config;
mod dry_run;
mod output;
mod output_types;
mod storage;
mod interactive;
mod progress;
mod errors;
mod auto_detect;

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

    // Create async runtime
    let runtime = tokio::runtime::Runtime::new()?;

    // Execute the command
    runtime.block_on(async {
        commands::execute(cli).await
    })?;

    Ok(())
}
