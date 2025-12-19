//! Command implementations

mod init;
mod add;
mod build;
mod query;
mod inspect;
mod status;

use crate::cli::{Cli, Commands};
use crate::output::OutputWriter;
use anyhow::Result;

/// Execute a CLI command
pub fn execute(cli: Cli) -> Result<()> {
    let output = OutputWriter::new(cli.json);

    match cli.command {
        Commands::Init(args) => init::execute(args, &output, cli.dry_run),
        Commands::Add(args) => add::execute(args, &output, cli.dry_run),
        Commands::Build(args) => build::execute(args, &output, cli.dry_run),
        Commands::Query(args) => query::execute(args, &output, cli.explain),
        Commands::Inspect(args) => inspect::execute(args, &output),
        Commands::Status(args) => status::execute(args, &output),
    }
}
