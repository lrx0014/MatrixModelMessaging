use clap::{Parser, Subcommand};
use std::{path::PathBuf, time::Duration};

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
    /// Controlled two-client protocol acceptance. Receives only user sessions.
    /// Terminate at an exact protocol persistence boundary in a disposable fixture.
    CrashProbe {
        #[arg(long)]
        homeserver: String,
        #[arg(long)]
        data: PathBuf,
        #[arg(long)]
        session: PathBuf,
        #[arg(long)]
        point: m3_agent::matrix::CrashPoint,
    },
    Console {
        #[arg(long)]
        homeserver: String,
        #[arg(long)]
        data: PathBuf,
        #[arg(long)]
        session: PathBuf,
    },
    ProtocolSmoke {
        #[arg(long)]
        homeserver: String,
        #[arg(long)]
        sessions: PathBuf,
        #[arg(long)]
        state: PathBuf,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    match Args::parse().command {
        Command::CrashProbe {
            homeserver,
            data,
            session,
            point,
        } => {
            let runner = m3_agent::matrix::PersistentClient::open_with_crash(
                &homeserver,
                &data,
                &session,
                Some(point),
            )
            .await?;
            runner.sync_once().await?;
            anyhow::bail!("Crash boundary was not reached");
        }
        Command::Console {
            homeserver,
            data,
            session,
        } => m3_agent::console::run(&homeserver, &data, &session).await?,
        Command::ProtocolSmoke {
            homeserver,
            sessions,
            state,
        } => {
            tokio::time::timeout(
                Duration::from_secs(180),
                m3_agent::probe::run(&homeserver, &sessions, &state),
            )
            .await??;
        }
        Command::StorageSmoke => {
            let version = m3_agent::storage_smoke().await?;
            println!(
                "SDK state/crypto stores and SQLx FTS5 passed (SQLite {version}); no Matrix interoperability claim"
            );
        }
    }
    Ok(())
}
