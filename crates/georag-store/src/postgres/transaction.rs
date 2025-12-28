use georag_core::error::{GeoragError, Result};
use sqlx::{PgPool, Postgres, Transaction as SqlxTransaction};
use std::time::Duration;
use tokio::time::timeout;

/// Transaction wrapper that provides ACID guarantees
pub struct Transaction<'a> {
    inner: Option<SqlxTransaction<'a, Postgres>>,
    timeout_duration: Duration,
}

impl<'a> Transaction<'a> {
    /// Create a new transaction from a sqlx transaction
    fn new(tx: SqlxTransaction<'a, Postgres>, timeout_duration: Duration) -> Self {
        Self { inner: Some(tx), timeout_duration }
    }

    /// Get a mutable reference to the inner transaction
    ///
    /// This allows executing queries within the transaction context
    pub fn inner_mut(&mut self) -> Result<&mut SqlxTransaction<'a, Postgres>> {
        self.inner
            .as_mut()
            .ok_or_else(|| GeoragError::Serialization("Transaction already completed".to_string()))
    }

    /// Commit the transaction, making all changes permanent
    pub async fn commit(mut self) -> Result<()> {
        let tx = self.inner.take().ok_or_else(|| {
            GeoragError::Serialization("Transaction already completed".to_string())
        })?;

        // Apply timeout to commit operation
        let commit_result = timeout(self.timeout_duration, tx.commit()).await;

        match commit_result {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => {
                Err(GeoragError::Serialization(format!("Failed to commit transaction: {}", e)))
            }
            Err(_) => Err(GeoragError::Serialization(format!(
                "Transaction commit timeout after {}s",
                self.timeout_duration.as_secs()
            ))),
        }
    }

    /// Rollback the transaction, discarding all changes
    pub async fn rollback(mut self) -> Result<()> {
        let tx = self.inner.take().ok_or_else(|| {
            GeoragError::Serialization("Transaction already completed".to_string())
        })?;

        // Apply timeout to rollback operation
        let rollback_result = timeout(self.timeout_duration, tx.rollback()).await;

        match rollback_result {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => {
                Err(GeoragError::Serialization(format!("Failed to rollback transaction: {}", e)))
            }
            Err(_) => Err(GeoragError::Serialization(format!(
                "Transaction rollback timeout after {}s",
                self.timeout_duration.as_secs()
            ))),
        }
    }
}

impl<'a> Drop for Transaction<'a> {
    /// Automatically rollback if transaction is dropped without explicit commit/rollback
    ///
    /// This ensures resources are properly cleaned up and prevents partial commits
    fn drop(&mut self) {
        if self.inner.is_some() {
            // Transaction will be automatically rolled back by sqlx when dropped
            // We don't need to do anything explicit here
        }
    }
}

/// Transaction manager for PostgresStore
///
/// Provides methods to begin transactions with configurable timeouts
pub struct TransactionManager {
    pool: PgPool,
    default_timeout: Duration,
}

impl TransactionManager {
    /// Create a new transaction manager
    pub fn new(pool: PgPool, default_timeout: Duration) -> Self {
        Self { pool, default_timeout }
    }

    /// Begin a new transaction with the default timeout
    pub async fn begin_transaction(&self) -> Result<Transaction<'_>> {
        self.begin_transaction_with_timeout(self.default_timeout).await
    }

    /// Begin a new transaction with a custom timeout
    pub async fn begin_transaction_with_timeout(
        &self,
        timeout_duration: Duration,
    ) -> Result<Transaction<'_>> {
        let tx = self.pool.begin().await.map_err(|e| {
            GeoragError::Serialization(format!("Failed to begin transaction: {}", e))
        })?;

        Ok(Transaction::new(tx, timeout_duration))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_transaction_manager_creation() {
        let pool = PgPool::connect_lazy("postgresql://localhost/test").unwrap();
        let manager = TransactionManager::new(pool, Duration::from_secs(30));
        assert_eq!(manager.default_timeout, Duration::from_secs(30));
    }
}
