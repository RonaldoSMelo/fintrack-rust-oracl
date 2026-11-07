use std::sync::Arc;

use color_eyre::eyre::WrapErr;
use oracledb::{Connection, ErrorKind, Pool, PoolConfig};
use thiserror::Error;
use tokio::task;

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("{0}")]
    Oracle(oracledb::Error),
    #[error(transparent)]
    Worker(#[from] task::JoinError),
}

impl From<oracledb::Error> for DatabaseError {
    fn from(error: oracledb::Error) -> Self {
        Self::Oracle(error)
    }
}

impl DatabaseError {
    pub fn is_unique_violation(&self) -> bool {
        matches!(self, Self::Oracle(error)
            if matches!(error.kind(), ErrorKind::DbError(db_error) if db_error.code() == 1))
    }
}

#[derive(Clone)]
pub struct Database(Arc<Pool>);

impl Database {
    pub async fn from_env() -> color_eyre::Result<Self> {
        let user = std::env::var("ORACLE_USER").wrap_err("Set ORACLE_USER")?;
        let password = std::env::var("ORACLE_PASSWORD").wrap_err("Set ORACLE_PASSWORD")?;
        let connect = std::env::var("ORACLE_CONNECT").wrap_err("Set ORACLE_CONNECT")?;

        let pool = task::spawn_blocking(move || {
            let config = PoolConfig::default()
                .set_credentials(&user, &password)
                .set_connect_string(&connect)?
                .set_max_connections(8);
            let pool = oracledb::create_pool(config)?;
            pool.acquire()?.query_row("SELECT 1 FROM dual", &[])?;
            Ok::<_, oracledb::Error>(pool)
        })
        .await?
        .map_err(DatabaseError::from)
        .wrap_err("Could not connect to Oracle")?;

        Ok(Self(Arc::new(pool)))
    }

    pub async fn run<T: Send + 'static>(
        &self,
        operation: impl FnOnce(&Connection) -> Result<T, oracledb::Error> + Send + 'static,
    ) -> Result<T, DatabaseError> {
        let pool = Arc::clone(&self.0);
        Ok(task::spawn_blocking(move || {
            let connection = pool.acquire()?;
            let result = operation(&connection);
            if result.is_err() {
                if let Err(error) = connection.rollback() {
                    tracing::error!(%error, "Oracle rollback failed");
                }
            }
            result
        })
        .await??)
    }
}
