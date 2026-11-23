use anyhow::ensure;
use m3_storage::{DatabaseKind, open, queries};
use matrix_sdk::Client;

/// Validate SDK-owned stores and a separate application database in one process.
/// All files are disposable. This does not log in or run Client-Server sync.
pub async fn storage_smoke() -> anyhow::Result<String> {
    let directory = tempfile::tempdir()?;
    let sdk_path = directory.path().join("sdk");
    let client = Client::builder()
        .homeserver_url("http://127.0.0.1:9")
        // Fixed fixture capability avoids a network /versions request during build.
        .server_versions([matrix_sdk::ruma::api::MatrixVersion::V1_0])
        .sqlite_store(&sdk_path, None)
        .build()
        .await?;
    let pool = open(
        &directory.path().join("application.sqlite3"),
        DatabaseKind::Agent,
    )
    .await?;
    let mut connection = pool.acquire().await?;
    let journal_mode: String = sqlx::query_scalar("PRAGMA journal_mode")
        .fetch_one(&mut *connection)
        .await?;
    let foreign_keys: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
        .fetch_one(&mut *connection)
        .await?;
    let synchronous: i64 = sqlx::query_scalar("PRAGMA synchronous")
        .fetch_one(&mut *connection)
        .await?;
    let busy_timeout: i64 = sqlx::query_scalar("PRAGMA busy_timeout")
        .fetch_one(&mut *connection)
        .await?;
    ensure!(
        journal_mode == "wal",
        "Application database did not enable WAL"
    );
    ensure!(
        foreign_keys == 1,
        "Application database did not enable foreign keys"
    );
    ensure!(
        synchronous == 2,
        "Application database did not enable FULL durability"
    );
    ensure!(
        busy_timeout == 5_000,
        "Application database busy timeout differs"
    );
    let version: String = sqlx::query_scalar("SELECT sqlite_version()")
        .fetch_one(&mut *connection)
        .await?;
    for (tokenizer, create, insert, search, term) in [
        (
            "unicode61",
            "CREATE VIRTUAL TABLE words USING fts5(body, tokenize='unicode61')",
            "INSERT INTO words(body) VALUES (?)",
            "SELECT count(*) FROM words WHERE words MATCH ?",
            "persistent",
        ),
        (
            "trigram",
            "CREATE VIRTUAL TABLE substrings USING fts5(body, tokenize='trigram')",
            "INSERT INTO substrings(body) VALUES (?)",
            "SELECT count(*) FROM substrings WHERE substrings MATCH ?",
            "sist",
        ),
    ] {
        sqlx::query(create).execute(&mut *connection).await?;
        sqlx::query(insert)
            .bind("persistent personal memory")
            .execute(&mut *connection)
            .await?;
        let count: i64 = sqlx::query_scalar(search)
            .bind(term)
            .fetch_one(&mut *connection)
            .await?;
        ensure!(count == 1, "FTS5 {tokenizer} query returned {count} rows");
    }
    connection.close().await?;
    queries::save_checkpoint(&pool, "smoke", "fixture-position", 1).await?;
    ensure!(
        queries::checkpoint(&pool, "smoke").await?.as_deref() == Some("fixture-position"),
        "Migrated checkpoint query failed"
    );
    pool.close().await;
    drop(client);
    // TempDir removes only this invocation's stores after their owners are dropped.
    directory.close()?;
    Ok(version)
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn sdk_and_application_sqlite_coexist_with_both_fts_tokenizers() {
        super::storage_smoke().await.unwrap();
    }
}
