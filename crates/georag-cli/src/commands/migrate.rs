use crate::cli::MigrateArgs;
use crate::config::load_workspace_config;
use crate::output::OutputWriter;
use anyhow::{Context, Result};
use georag_store::memory::{MemoryDocumentStore, MemorySpatialStore, MemoryVectorStore};
use georag_store::ports::{DocumentStore, SpatialStore, VectorStore};
use georag_store::postgres::{PostgresConfig, PostgresStore};
use std::path::PathBuf;
use std::time::Instant;

/// Progress information for migration
#[derive(Debug, Clone)]
pub struct MigrationProgress {
    pub datasets_total: usize,
    pub datasets_migrated: usize,
    pub features_total: usize,
    pub features_migrated: usize,
    pub chunks_total: usize,
    pub chunks_migrated: usize,
    pub embeddings_total: usize,
    pub embeddings_migrated: usize,
    pub elapsed_secs: u64,
}

impl MigrationProgress {
    fn new() -> Self {
        Self {
            datasets_total: 0,
            datasets_migrated: 0,
            features_total: 0,
            features_migrated: 0,
            chunks_total: 0,
            chunks_migrated: 0,
            embeddings_total: 0,
            embeddings_migrated: 0,
            elapsed_secs: 0,
        }
    }

    fn migrated_records(&self) -> usize {
        self.datasets_migrated
            + self.features_migrated
            + self.chunks_migrated
            + self.embeddings_migrated
    }
}

/// Execute the migrate command
pub fn execute(args: MigrateArgs, output: &OutputWriter, _dry_run: bool) -> Result<()> {
    // Load workspace configuration
    let workspace_root = PathBuf::from(".");
    let _config =
        load_workspace_config(&workspace_root).context("Failed to load workspace configuration")?;

    // Create runtime for async operations
    let runtime = tokio::runtime::Runtime::new().context("Failed to create async runtime")?;

    runtime.block_on(async { migrate_data(args, output).await })
}

async fn migrate_data(args: MigrateArgs, output: &OutputWriter) -> Result<()> {
    let start_time = Instant::now();
    let mut progress = MigrationProgress::new();

    output.info("Loading data from in-memory storage...");

    let source_spatial = MemorySpatialStore::new();
    let source_vector = MemoryVectorStore::new();
    let source_document = MemoryDocumentStore::new();

    output.info("Initializing PostgreSQL connection...");

    // Initialize destination (PostgreSQL) store
    let pg_config = PostgresConfig::from_database_url(&args.database_url)?;
    let dest_store = if args.dry_run {
        output.info("DRY RUN: Would connect to PostgreSQL");
        None
    } else {
        let store = PostgresStore::with_migrations(pg_config)
            .await
            .context("Failed to initialize PostgreSQL store")?;
        output.success("Connected to PostgreSQL");
        Some(store)
    };

    // Count total records
    output.info("Counting records in source storage...");
    progress.datasets_total = source_spatial.list_datasets().await?.len();
    progress.chunks_total = source_document.list_chunk_ids().await?.len();

    // Count features by iterating through datasets
    let datasets = source_spatial.list_datasets().await?;
    for dataset_meta in &datasets {
        if let Some(dataset) = source_spatial.get_dataset(dataset_meta.id).await? {
            progress.features_total += dataset.feature_count;
        }
    }

    // Count embeddings by checking each chunk
    let chunk_ids = source_document.list_chunk_ids().await?;
    for chunk_id in &chunk_ids {
        if source_vector.get_embedding(*chunk_id).await?.is_some() {
            progress.embeddings_total += 1;
        }
    }

    output.info(format!(
        "Found {} datasets, {} features, {} chunks, {} embeddings",
        progress.datasets_total,
        progress.features_total,
        progress.chunks_total,
        progress.embeddings_total
    ));

    if args.dry_run {
        output.info("DRY RUN: Would migrate the following:");
        output.info(format!("  - {} datasets", progress.datasets_total));
        output.info(format!("  - {} features", progress.features_total));
        output.info(format!("  - {} chunks", progress.chunks_total));
        output.info(format!("  - {} embeddings", progress.embeddings_total));
        return Ok(());
    }

    let dest_store = dest_store.unwrap();

    // Migrate datasets and features
    if progress.datasets_total > 0 {
        output.info("Migrating datasets and features...");
        migrate_datasets_and_features(
            &source_spatial,
            &dest_store,
            &mut progress,
            args.batch_size,
            output,
        )
        .await?;
    }

    // Migrate chunks
    if progress.chunks_total > 0 {
        output.info("Migrating chunks...");
        migrate_chunks(&source_document, &dest_store, &mut progress, args.batch_size, output)
            .await?;
    }

    // Migrate embeddings
    if progress.embeddings_total > 0 {
        output.info("Migrating embeddings...");
        migrate_embeddings(
            &source_vector,
            &source_document,
            &dest_store,
            &mut progress,
            args.batch_size,
            output,
        )
        .await?;
    }

    progress.elapsed_secs = start_time.elapsed().as_secs();

    // Verify integrity if requested
    if args.verify {
        output.info("Verifying data integrity...");
        verify_migration(&dest_store, &progress, output).await?;
    }

    // Report final progress
    output.success(format!(
        "Migration complete! Migrated {} records in {} seconds",
        progress.migrated_records(),
        progress.elapsed_secs
    ));
    output.info(format!("  - {} datasets", progress.datasets_migrated));
    output.info(format!("  - {} features", progress.features_migrated));
    output.info(format!("  - {} chunks", progress.chunks_migrated));
    output.info(format!("  - {} embeddings", progress.embeddings_migrated));

    Ok(())
}

async fn migrate_datasets_and_features(
    source: &MemorySpatialStore,
    dest: &PostgresStore,
    progress: &mut MigrationProgress,
    _batch_size: usize,
    output: &OutputWriter,
) -> Result<()> {
    let datasets = source.list_datasets().await?;

    for dataset_meta in datasets {
        // Get full dataset
        let dataset = source.get_dataset(dataset_meta.id).await?.context("Dataset not found")?;

        // Store dataset in destination
        let _new_id = dest.store_dataset(&dataset).await?;
        progress.datasets_migrated += 1;

        output.info(format!(
            "  Migrated dataset: {} ({} features)",
            dataset.name, dataset.feature_count
        ));

        progress.features_migrated += dataset.feature_count;
    }

    Ok(())
}

async fn migrate_chunks(
    source: &MemoryDocumentStore,
    dest: &PostgresStore,
    progress: &mut MigrationProgress,
    batch_size: usize,
    output: &OutputWriter,
) -> Result<()> {
    let chunk_ids = source.list_chunk_ids().await?;
    let total_chunks = chunk_ids.len();

    for (i, chunk_batch) in chunk_ids.chunks(batch_size).enumerate() {
        let chunks = source.get_chunks(chunk_batch).await?;
        dest.store_chunks(&chunks).await?;

        progress.chunks_migrated += chunks.len();

        if (i + 1) % 10 == 0 || progress.chunks_migrated == total_chunks {
            output.info(format!(
                "  Progress: {}/{} chunks ({:.1}%)",
                progress.chunks_migrated,
                total_chunks,
                (progress.chunks_migrated as f64 / total_chunks as f64) * 100.0
            ));
        }
    }

    Ok(())
}

async fn migrate_embeddings(
    source: &MemoryVectorStore,
    doc_store: &MemoryDocumentStore,
    dest: &PostgresStore,
    progress: &mut MigrationProgress,
    batch_size: usize,
    output: &OutputWriter,
) -> Result<()> {
    // Get all chunk IDs that have embeddings
    let chunk_ids = doc_store.list_chunk_ids().await?;

    let mut embeddings_to_migrate = Vec::new();
    for chunk_id in chunk_ids {
        if let Some(embedding) = source.get_embedding(chunk_id).await? {
            embeddings_to_migrate.push(embedding);
        }
    }

    let total_embeddings = embeddings_to_migrate.len();

    for (i, embedding_batch) in embeddings_to_migrate.chunks(batch_size).enumerate() {
        dest.store_embeddings(embedding_batch).await?;

        progress.embeddings_migrated += embedding_batch.len();

        if (i + 1) % 10 == 0 || progress.embeddings_migrated == total_embeddings {
            output.info(format!(
                "  Progress: {}/{} embeddings ({:.1}%)",
                progress.embeddings_migrated,
                total_embeddings,
                (progress.embeddings_migrated as f64 / total_embeddings as f64) * 100.0
            ));
        }
    }

    Ok(())
}

async fn verify_migration(
    dest: &PostgresStore,
    progress: &MigrationProgress,
    output: &OutputWriter,
) -> Result<()> {
    output.info("Verifying migration integrity...");

    // Count records in destination
    let dest_datasets = dest.list_datasets().await?.len();
    let dest_chunks = dest.list_chunk_ids().await?.len();

    // Count embeddings in destination
    let dest_embeddings = {
        let chunk_ids = dest.list_chunk_ids().await?;
        let mut count = 0;
        for chunk_id in chunk_ids {
            if dest.get_embedding(chunk_id).await?.is_some() {
                count += 1;
            }
        }
        count
    };

    // Verify counts match
    let mut errors = Vec::new();

    if dest_datasets != progress.datasets_migrated {
        errors.push(format!(
            "Dataset count mismatch: expected {}, found {}",
            progress.datasets_migrated, dest_datasets
        ));
    }

    if dest_chunks != progress.chunks_migrated {
        errors.push(format!(
            "Chunk count mismatch: expected {}, found {}",
            progress.chunks_migrated, dest_chunks
        ));
    }

    if dest_embeddings != progress.embeddings_migrated {
        errors.push(format!(
            "Embedding count mismatch: expected {}, found {}",
            progress.embeddings_migrated, dest_embeddings
        ));
    }

    if !errors.is_empty() {
        output.info("Integrity verification failed:");
        for error in &errors {
            output.info(format!("  - {}", error));
        }
        anyhow::bail!("Data integrity verification failed");
    }

    output.success("Data integrity verified!");
    output.info(format!("  ✓ {} datasets", dest_datasets));
    output.info(format!("  ✓ {} chunks", dest_chunks));
    output.info(format!("  ✓ {} embeddings", dest_embeddings));

    Ok(())
}
