use std::{path::Path, time::Duration};

use sqlx::{
    SqlitePool,
    migrate::Migrator,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
};

pub static CONTROL_MIGRATIONS: Migrator = sqlx::migrate!("./migrations/control");
pub static AGENT_MIGRATIONS: Migrator = sqlx::migrate!("./migrations/agent");

#[derive(Clone, Copy)]
pub enum DatabaseKind {
    Control,
    Agent,
}

impl DatabaseKind {
    pub fn migrator(self) -> &'static Migrator {
        match self {
            Self::Control => &CONTROL_MIGRATIONS,
            Self::Agent => &AGENT_MIGRATIONS,
        }
    }
}

/// Application data only. Never pass an SDK-owned database path here.
/// One connection serializes all writes; add dedicated readers when needed.
pub async fn open(path: &Path, kind: DatabaseKind) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true)
        .synchronous(SqliteSynchronous::Full)
        .busy_timeout(Duration::from_secs(5));
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    if let Err(error) = kind.migrator().run(&pool).await {
        pool.close().await;
        return Err(sqlx::Error::Migrate(Box::new(error)));
    }
    Ok(pool)
}

// xtask can initialize preparation databases before query metadata exists.
#[cfg(feature = "queries")]
pub mod queries {
    use sqlx::SqlitePool;

    pub async fn global_frozen(pool: &SqlitePool) -> Result<bool, sqlx::Error> {
        let value = sqlx::query_scalar!("SELECT global_frozen FROM instance_state WHERE id = 1")
            .fetch_one(pool)
            .await?;
        Ok(value != 0)
    }

    pub async fn checkpoint(
        pool: &SqlitePool,
        stream: &str,
    ) -> Result<Option<String>, sqlx::Error> {
        sqlx::query_scalar!("SELECT position FROM checkpoints WHERE stream = ?", stream)
            .fetch_optional(pool)
            .await
    }

    /// Foundation query fixture. A future sync worker must couple checkpoints
    /// and durable handoff in one transaction; this is not a recovery implementation.
    pub async fn save_checkpoint(
        pool: &SqlitePool,
        stream: &str,
        position: &str,
        committed_at: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO checkpoints (stream, position, committed_at) VALUES (?, ?, ?) ON CONFLICT(stream) DO UPDATE SET position = excluded.position, committed_at = excluded.committed_at",
            stream, position, committed_at
        ).execute(pool).await?;
        Ok(())
    }
}

#[cfg(all(test, feature = "queries"))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn migrations_preserve_state_on_reopen_and_keep_histories_separate() {
        let dir = tempfile::tempdir().unwrap();
        let control_path = dir.path().join("control.sqlite3");
        let agent_path = dir.path().join("agent.sqlite3");
        let control = open(&control_path, DatabaseKind::Control).await.unwrap();
        let agent = open(&agent_path, DatabaseKind::Agent).await.unwrap();
        assert!(!queries::global_frozen(&control).await.unwrap());
        sqlx::query("UPDATE instance_state SET global_frozen = 1 WHERE id = 1")
            .execute(&control)
            .await
            .unwrap();
        queries::save_checkpoint(&agent, "fixture", "first", 1)
            .await
            .unwrap();
        queries::save_checkpoint(&agent, "fixture", "second", 2)
            .await
            .unwrap();
        assert!(queries::checkpoint(&control, "fixture").await.is_err());
        assert!(queries::global_frozen(&agent).await.is_err());
        control.close().await;
        agent.close().await;
        let control = open(&control_path, DatabaseKind::Control).await.unwrap();
        let agent = open(&agent_path, DatabaseKind::Agent).await.unwrap();
        assert!(queries::global_frozen(&control).await.unwrap());
        assert_eq!(
            queries::checkpoint(&agent, "fixture")
                .await
                .unwrap()
                .as_deref(),
            Some("second")
        );
        assert_eq!(queries::checkpoint(&agent, "missing").await.unwrap(), None);
        assert!(
            sqlx::query("UPDATE instance_state SET global_frozen = 2")
                .execute(&control)
                .await
                .is_err()
        );
        control.close().await;
        agent.close().await;
    }
}
