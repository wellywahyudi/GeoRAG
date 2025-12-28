use sqlx::PgPool;
use thiserror::Error;

/// Migration error types
#[derive(Debug, Error)]
pub enum MigrationError {
    #[error("Migration failed: {0}")]
    Failed(#[from] sqlx::migrate::MigrateError),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

/// Migration status information
#[derive(Debug, Clone)]
pub struct MigrationStatus {
    /// Version number of the migration
    pub version: i64,
    /// Description of the migration
    pub description: String,
    /// Whether the migration has been applied
    pub applied: bool,
    /// Checksum of the migration file
    pub checksum: Vec<u8>,
}

/// Migration manager for handling database schema migrations
pub struct MigrationManager {
    pool: PgPool,
}

impl MigrationManager {
    /// Create a new migration manager
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Run all pending migrations
    ///
    /// This applies all migrations that haven't been applied yet,
    /// in version order. Uses sqlx's built-in migration system.
    pub async fn run_migrations(&self) -> Result<(), MigrationError> {
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .map_err(MigrationError::Failed)?;
        Ok(())
    }

    /// Check migration status
    pub async fn check_status(&self) -> Result<Vec<MigrationStatus>, MigrationError> {
        // Get the migrator
        let migrator = sqlx::migrate!("./migrations");

        // Query applied migrations from the database
        let applied_migrations: Vec<(i64, Vec<u8>)> =
            sqlx::query_as("SELECT version, checksum FROM _sqlx_migrations ORDER BY version")
                .fetch_all(&self.pool)
                .await
                .unwrap_or_default();

        let applied_versions: std::collections::HashSet<i64> =
            applied_migrations.iter().map(|(v, _)| *v).collect();

        // Build status for all migrations
        let mut statuses = Vec::new();
        for migration in migrator.iter() {
            statuses.push(MigrationStatus {
                version: migration.version,
                description: migration.description.to_string(),
                applied: applied_versions.contains(&migration.version),
                checksum: migration.checksum.to_vec(),
            });
        }

        Ok(statuses)
    }

    /// Check if there are pending migrations
    pub async fn has_pending_migrations(&self) -> Result<bool, MigrationError> {
        let status = self.check_status().await?;
        Ok(status.iter().any(|s| !s.applied))
    }

    /// Get the current schema version (highest applied migration)
    pub async fn current_version(&self) -> Result<Option<i64>, MigrationError> {
        let version: Option<(i64,)> =
            sqlx::query_as("SELECT version FROM _sqlx_migrations ORDER BY version DESC LIMIT 1")
                .fetch_optional(&self.pool)
                .await?;

        Ok(version.map(|(v,)| v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_status_creation() {
        let status = MigrationStatus {
            version: 1,
            description: "Initial schema".to_string(),
            applied: true,
            checksum: vec![1, 2, 3],
        };

        assert_eq!(status.version, 1);
        assert_eq!(status.description, "Initial schema");
        assert!(status.applied);
        assert_eq!(status.checksum, vec![1, 2, 3]);
    }
}
