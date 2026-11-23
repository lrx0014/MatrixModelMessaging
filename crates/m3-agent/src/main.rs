use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    version,
    about = "M3 Runner foundation; social runtime is not implemented yet"
)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Open disposable SDK and application stores and verify native SQLite/FTS5.
    /// No account, credentials, or homeserver connection is used.
    StorageSmoke,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    match Args::parse().command {
        Command::StorageSmoke => {
            let version = m3_agent::storage_smoke().await?;
            println!(
                "SDK state/crypto stores and SQLx FTS5 passed (SQLite {version}); no Matrix interoperability claim"
            );
        }
    }
    Ok(())
}
