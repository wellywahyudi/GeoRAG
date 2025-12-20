//! Index management operations

use sqlx::PgPool;
use georag_core::error::{Result, GeoragError};
use std::time::Instant;

/// Result of an index rebuild operation
#[derive(Debug, Clone)]
pub struct RebuildResult {
    /// Number of indexes rebuilt
    pub indexes_rebuilt: usize,
    /// Duration in seconds
    pub duration_secs: f64,
    /// Any warnings encountered
    pub warnings: Vec<String>,
}

/// Statistics for a database index
#[derive(Debug, Clone)]
pub struct IndexStats {
    /// Name of the index
    pub index_name: String,
    /// Name of the table the index belongs to
    pub table_name: String,
    /// Type of index (btree, gist, ivfflat, etc.)
    pub index_type: String,
    /// Size of the index in bytes
    pub size_bytes: i64,
    /// Number of rows in the table
    pub row_count: i64,
    /// Last vacuum time (if available)
    pub last_vacuum: Option<String>,
    /// Last analyze time (if available)
    pub last_analyze: Option<String>,
}

/// Result of a vacuum operation
#[derive(Debug, Clone)]
pub struct VacuumResult {
    /// Number of tables processed
    pub tables_processed: usize,
    /// Duration in seconds
    pub duration_secs: f64,
    /// Any warnings encountered
    pub warnings: Vec<String>,
}

/// Rebuild database indexes
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `index_name` - Optional specific index to rebuild (rebuilds all if None)
/// * `concurrently` - Whether to rebuild concurrently (non-blocking)
///
/// # Requirements
/// - 10.1: Rebuild all spatial and vector indexes
/// - 10.5: Rebuild without blocking read operations (CONCURRENTLY)
pub async fn rebuild_indexes(
    pool: &PgPool,
    index_name: Option<&str>,
    concurrently: bool,
) -> Result<RebuildResult> {
    let start = Instant::now();
    let mut warnings = Vec::new();
    let mut indexes_rebuilt = 0;

    // Get list of indexes to rebuild
    let indexes = if let Some(name) = index_name {
        // Rebuild specific index
        vec![name.to_string()]
    } else {
        // Get all GeoRAG indexes
        get_georag_indexes(pool).await?
    };

    for index in &indexes {
        match rebuild_single_index(pool, index, concurrently).await {
            Ok(_) => {
                indexes_rebuilt += 1;
            }
            Err(e) => {
                warnings.push(format!("Failed to rebuild index {}: {}", index, e));
            }
        }
    }

    let duration_secs = start.elapsed().as_secs_f64();

    Ok(RebuildResult {
        indexes_rebuilt,
        duration_secs,
        warnings,
    })
}

/// Get list of GeoRAG-related indexes
async fn get_georag_indexes(pool: &PgPool) -> Result<Vec<String>> {
    let query = r#"
        SELECT indexname
        FROM pg_indexes
        WHERE schemaname = 'public'
        AND (
            indexname LIKE 'idx_%'
            OR indexname LIKE '%_pkey'
        )
        ORDER BY indexname
    "#;

    let rows = sqlx::query_scalar::<_, String>(query)
        .fetch_all(pool)
        .await
        .map_err(|e| GeoragError::Serialization(format!("Failed to get index list: {}", e)))?;

    Ok(rows)
}

/// Rebuild a single index
async fn rebuild_single_index(
    pool: &PgPool,
    index_name: &str,
    concurrently: bool,
) -> Result<()> {
    let concurrent_clause = if concurrently { "CONCURRENTLY" } else { "" };
    
    let query = format!("REINDEX INDEX {} {}", concurrent_clause, index_name);
    
    sqlx::query(&query)
        .execute(pool)
        .await
        .map_err(|e| GeoragError::Serialization(format!("Failed to rebuild index: {}", e)))?;

    Ok(())
}

/// Get statistics for database indexes
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `index_name` - Optional specific index to get stats for (gets all if None)
///
/// # Requirements
/// - 10.2: Report index size, row count, and last vacuum time
pub async fn get_index_stats(
    pool: &PgPool,
    index_name: Option<&str>,
) -> Result<Vec<IndexStats>> {
    let query = if let Some(name) = index_name {
        format!(
            r#"
            SELECT
                i.indexname as index_name,
                i.tablename as table_name,
                am.amname as index_type,
                pg_relation_size(quote_ident(i.schemaname) || '.' || quote_ident(i.indexname)) as size_bytes,
                c.reltuples::bigint as row_count,
                pg_stat_get_last_vacuum_time(c.oid)::text as last_vacuum,
                pg_stat_get_last_analyze_time(c.oid)::text as last_analyze
            FROM pg_indexes i
            JOIN pg_class c ON c.relname = i.tablename
            JOIN pg_am am ON am.oid = c.relam
            WHERE i.schemaname = 'public'
            AND i.indexname = '{}'
            ORDER BY i.indexname
            "#,
            name
        )
    } else {
        r#"
            SELECT
                i.indexname as index_name,
                i.tablename as table_name,
                am.amname as index_type,
                pg_relation_size(quote_ident(i.schemaname) || '.' || quote_ident(i.indexname)) as size_bytes,
                c.reltuples::bigint as row_count,
                pg_stat_get_last_vacuum_time(c.oid)::text as last_vacuum,
                pg_stat_get_last_analyze_time(c.oid)::text as last_analyze
            FROM pg_indexes i
            JOIN pg_class c ON c.relname = i.tablename
            JOIN pg_am am ON am.oid = c.relam
            WHERE i.schemaname = 'public'
            AND (i.indexname LIKE 'idx_%' OR i.indexname LIKE '%_pkey')
            ORDER BY i.indexname
        "#.to_string()
    };

    let rows = sqlx::query_as::<_, (String, String, String, i64, i64, Option<String>, Option<String>)>(&query)
        .fetch_all(pool)
        .await
        .map_err(|e| GeoragError::Serialization(format!("Failed to get index stats: {}", e)))?;

    let stats = rows
        .into_iter()
        .map(|(index_name, table_name, index_type, size_bytes, row_count, last_vacuum, last_analyze)| {
            IndexStats {
                index_name,
                table_name,
                index_type,
                size_bytes,
                row_count,
                last_vacuum,
                last_analyze,
            }
        })
        .collect();

    Ok(stats)
}

/// Run VACUUM and optionally ANALYZE on database tables
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `table_name` - Optional specific table to vacuum (vacuums all if None)
/// * `analyze` - Whether to run ANALYZE after VACUUM
/// * `full` - Whether to run FULL vacuum (locks table, reclaims more space)
///
/// # Requirements
/// - 10.4: Support VACUUM and ANALYZE operations
pub async fn vacuum_analyze(
    pool: &PgPool,
    table_name: Option<&str>,
    analyze: bool,
    full: bool,
) -> Result<VacuumResult> {
    let start = Instant::now();
    let mut warnings = Vec::new();
    let mut tables_processed = 0;

    // Get list of tables to vacuum
    let tables = if let Some(name) = table_name {
        vec![name.to_string()]
    } else {
        get_georag_tables(pool).await?
    };

    for table in &tables {
        match vacuum_single_table(pool, table, analyze, full).await {
            Ok(_) => {
                tables_processed += 1;
            }
            Err(e) => {
                warnings.push(format!("Failed to vacuum table {}: {}", table, e));
            }
        }
    }

    let duration_secs = start.elapsed().as_secs_f64();

    Ok(VacuumResult {
        tables_processed,
        duration_secs,
        warnings,
    })
}

/// Get list of GeoRAG tables
async fn get_georag_tables(pool: &PgPool) -> Result<Vec<String>> {
    let query = r#"
        SELECT tablename
        FROM pg_tables
        WHERE schemaname = 'public'
        AND tablename IN (
            'workspaces', 'datasets', 'features', 'documents',
            'chunks', 'embeddings', 'index_builds'
        )
        ORDER BY tablename
    "#;

    let rows = sqlx::query_scalar::<_, String>(query)
        .fetch_all(pool)
        .await
        .map_err(|e| GeoragError::Serialization(format!("Failed to get table list: {}", e)))?;

    Ok(rows)
}

/// Vacuum a single table
async fn vacuum_single_table(
    pool: &PgPool,
    table_name: &str,
    analyze: bool,
    full: bool,
) -> Result<()> {
    let mut parts = vec!["VACUUM"];
    
    if full {
        parts.push("FULL");
    }
    
    if analyze {
        parts.push("ANALYZE");
    }
    
    parts.push(table_name);
    
    let query = parts.join(" ");
    
    sqlx::query(&query)
        .execute(pool)
        .await
        .map_err(|e| GeoragError::Serialization(format!("Failed to vacuum table: {}", e)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rebuild_result_creation() {
        let result = RebuildResult {
            indexes_rebuilt: 5,
            duration_secs: 1.23,
            warnings: vec!["test warning".to_string()],
        };
        
        assert_eq!(result.indexes_rebuilt, 5);
        assert_eq!(result.duration_secs, 1.23);
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn test_index_stats_creation() {
        let stats = IndexStats {
            index_name: "idx_test".to_string(),
            table_name: "test_table".to_string(),
            index_type: "btree".to_string(),
            size_bytes: 1024,
            row_count: 100,
            last_vacuum: Some("2024-01-01".to_string()),
            last_analyze: Some("2024-01-01".to_string()),
        };
        
        assert_eq!(stats.index_name, "idx_test");
        assert_eq!(stats.size_bytes, 1024);
    }

    #[test]
    fn test_vacuum_result_creation() {
        let result = VacuumResult {
            tables_processed: 3,
            duration_secs: 2.45,
            warnings: vec![],
        };
        
        assert_eq!(result.tables_processed, 3);
        assert_eq!(result.duration_secs, 2.45);
        assert!(result.warnings.is_empty());
    }
}
