use crate::cli::{DbArgs, DbCommand};
use crate::output::OutputWriter;
use anyhow::{Context, Result};
use georag_store::postgres::{PostgresConfig, PostgresStore};

/// Execute database management commands
pub fn execute(args: DbArgs, output: &OutputWriter, dry_run: bool) -> Result<()> {
    let rt = tokio::runtime::Runtime::new().context("Failed to create async runtime")?;

    rt.block_on(async {
        // Load PostgreSQL configuration
        let config = PostgresConfig::from_env()
            .context("Failed to load database configuration. Ensure DATABASE_URL is set.")?;

        // Create store connection
        let store = PostgresStore::new(config).await.context("Failed to connect to database")?;

        match args.command {
            DbCommand::Rebuild(rebuild_args) => {
                execute_rebuild(&store, rebuild_args, output, dry_run).await
            }
            DbCommand::Stats(stats_args) => execute_stats(&store, stats_args, output).await,
            DbCommand::Vacuum(vacuum_args) => {
                execute_vacuum(&store, vacuum_args, output, dry_run).await
            }
        }
    })
}

/// Execute index rebuild command
async fn execute_rebuild(
    store: &PostgresStore,
    args: crate::cli::RebuildArgs,
    output: &OutputWriter,
    dry_run: bool,
) -> Result<()> {
    if dry_run {
        output.info("Dry run: Would rebuild indexes");
        if args.concurrently {
            output.info("  - Using CONCURRENTLY option (non-blocking)");
        }
        if let Some(ref index_name) = args.index {
            output.info(format!("  - Target index: {}", index_name));
        } else {
            output.info("  - Target: All indexes");
        }
        return Ok(());
    }

    output.info("Rebuilding indexes...");

    let result = store
        .rebuild_indexes(args.index.as_deref(), args.concurrently)
        .await
        .context("Failed to rebuild indexes")?;

    output.success(format!(
        "Successfully rebuilt {} index(es) in {:.2}s",
        result.indexes_rebuilt, result.duration_secs
    ));

    if !result.warnings.is_empty() {
        output.warning("Warnings:");
        for warning in &result.warnings {
            output.warning(format!("  - {}", warning));
        }
    }

    Ok(())
}

/// Execute index stats command
async fn execute_stats(
    store: &PostgresStore,
    args: crate::cli::StatsArgs,
    output: &OutputWriter,
) -> Result<()> {
    output.info("Fetching index statistics...");

    let stats = store
        .get_index_stats(args.index.as_deref())
        .await
        .context("Failed to get index statistics")?;

    if stats.is_empty() {
        output.warning("No indexes found");
        return Ok(());
    }

    output.info(format!("\nFound {} index(es):\n", stats.len()));

    for stat in stats {
        output.info(format!("Index: {}", stat.index_name));
        output.info(format!("  Table: {}", stat.table_name));
        output.info(format!("  Type: {}", stat.index_type));
        output.info(format!("  Size: {}", format_bytes(stat.size_bytes)));
        output.info(format!("  Rows: {}", stat.row_count));

        if let Some(last_vacuum) = stat.last_vacuum {
            output.info(format!("  Last Vacuum: {}", last_vacuum));
        }

        if let Some(last_analyze) = stat.last_analyze {
            output.info(format!("  Last Analyze: {}", last_analyze));
        }

        output.info("");
    }

    Ok(())
}

/// Execute vacuum/analyze command
async fn execute_vacuum(
    store: &PostgresStore,
    args: crate::cli::VacuumArgs,
    output: &OutputWriter,
    dry_run: bool,
) -> Result<()> {
    if dry_run {
        output.info("Dry run: Would run VACUUM");
        if args.analyze {
            output.info("  - With ANALYZE");
        }
        if args.full {
            output.info("  - FULL vacuum (locks table)");
        }
        if let Some(ref table) = args.table {
            output.info(format!("  - Target table: {}", table));
        } else {
            output.info("  - Target: All tables");
        }
        return Ok(());
    }

    output.info("Running VACUUM...");

    let result = store
        .vacuum_analyze(args.table.as_deref(), args.analyze, args.full)
        .await
        .context("Failed to run VACUUM")?;

    output.success(format!(
        "Successfully vacuumed {} table(s) in {:.2}s",
        result.tables_processed, result.duration_secs
    ));

    if !result.warnings.is_empty() {
        output.warning("Warnings:");
        for warning in &result.warnings {
            output.warning(format!("  - {}", warning));
        }
    }

    Ok(())
}

/// Format bytes into human-readable format
fn format_bytes(bytes: i64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];

    if bytes < 0 {
        return "N/A".to_string();
    }

    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    if unit_idx == 0 {
        format!("{} {}", bytes, UNITS[0])
    } else {
        format!("{:.2} {}", size, UNITS[unit_idx])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1536), "1.50 KB");
        assert_eq!(format_bytes(1048576), "1.00 MB");
        assert_eq!(format_bytes(1073741824), "1.00 GB");
        assert_eq!(format_bytes(-1), "N/A");
    }
}
