use std::{collections::BTreeMap, fs, path::Path, process::Command};

use anyhow::ensure;
use m3_storage::{DatabaseKind, open};
use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};

fn files(directory: &Path) -> anyhow::Result<BTreeMap<String, Vec<u8>>> {
    let mut result = BTreeMap::new();
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            result.insert(
                path.file_name().unwrap().to_string_lossy().into_owned(),
                fs::read(path)?,
            );
        }
    }
    Ok(result)
}

pub async fn prepare(check: bool) -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    // Exercise the actual independent migration histories before preparing queries.
    for (name, kind) in [
        ("control", DatabaseKind::Control),
        ("agent", DatabaseKind::Agent),
    ] {
        let pool = open(&temp.path().join(format!("{name}.sqlite3")), kind).await?;
        pool.close().await;
    }
    // Compile-time description database only: combine SQL, not migration histories.
    // This is never used as an application or SDK database.
    let path = temp.path().join("description.sqlite3");
    let options = SqliteConnectOptions::new()
        .filename(&path)
        .create_if_missing(true);
    let mut connection = SqliteConnection::connect_with(&options).await?;
    for kind in [DatabaseKind::Control, DatabaseKind::Agent] {
        for migration in kind.migrator().iter() {
            sqlx::raw_sql(migration.sql.clone())
                .execute(&mut connection)
                .await?;
        }
    }
    connection.close().await?;
    let generated = temp.path().join(".sqlx");
    fs::create_dir(&generated)?;
    super::run(
        Command::new("cargo")
            .current_dir(super::root())
            .args([
                "check",
                "--locked",
                "--package",
                "m3-storage",
                "--features",
                "queries",
                "--lib",
            ])
            .env("DATABASE_URL", format!("sqlite://{}", path.display()))
            .env("SQLX_OFFLINE", "false")
            .env("SQLX_OFFLINE_DIR", &generated),
    )?;
    let fresh = files(&generated)?;
    ensure!(
        !fresh.is_empty(),
        "SQLx did not generate query descriptions"
    );
    let committed = super::root().join(".sqlx");
    if check {
        ensure!(
            files(&committed)? == fresh,
            "SQLx metadata is stale; run cargo xtask sqlx"
        );
    } else {
        fs::create_dir_all(&committed)?;
        for name in files(&committed)?.keys() {
            fs::remove_file(committed.join(name))?;
        }
        for (name, data) in fresh {
            fs::write(committed.join(name), data)?;
        }
    }
    Ok(())
}
