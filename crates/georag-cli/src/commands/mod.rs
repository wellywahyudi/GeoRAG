mod add;
mod build;
mod db;
mod doctor;
mod init;
mod migrate;
mod query;
mod status;

use crate::cli::{Cli, Commands};
use crate::output::OutputWriter;
use crate::storage::Storage;
use anyhow::Result;

/// Execute a CLI command
pub async fn execute(cli: Cli) -> Result<()> {
    let output = OutputWriter::new(cli.json);

    // Create storage backend based on CLI flag
    let storage = Storage::new(cli.storage.clone()).await?;

    match cli.command {
        Commands::Init(args) => init::execute(args, &output, cli.dry_run),
        Commands::Add(args) => add::execute(args, &output, cli.dry_run, &storage).await,
        Commands::Build(args) => build::execute(args, &output, cli.dry_run, &storage).await,
        Commands::Query(args) => query::execute(args, &output, cli.explain, &storage).await,
        Commands::Status(args) => status::execute(args, &output),
        Commands::Migrate(args) => migrate::execute(args, &output, cli.dry_run),
        Commands::Db(args) => db::execute(args, &output, cli.dry_run),
        Commands::Doctor(args) => doctor::execute(args, &output),
    }
}
