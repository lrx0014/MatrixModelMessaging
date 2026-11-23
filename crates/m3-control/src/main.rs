use std::net::SocketAddr;

use clap::{Parser, Subcommand};
use std::{path::PathBuf, sync::Arc};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(
    version,
    about = "M3 Control foundation: liveness and readiness endpoints"
)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,
    #[arg(long)]
    as_config: Option<PathBuf>,
    #[arg(long, requires = "as_config")]
    database: Option<PathBuf>,
    #[arg(long, default_value = "127.0.0.1:8080")]
    listen: SocketAddr,
}

#[derive(Subcommand)]
enum Command {
    /// Create one AS account and user-scoped device session. Control-only.
    Provision {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        localpart: String,
        #[arg(long)]
        session_output: PathBuf,
        #[arg(long)]
        database: PathBuf,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    if let Some(Command::Provision {
        config,
        localpart,
        session_output,
        database,
    }) = args.command
    {
        let config = m3_control::provision::Config::load(&config)?;
        let pool = m3_storage::open(&database, m3_storage::DatabaseKind::Control).await?;
        m3_control::provision::provision(&config, &localpart, &session_output).await?;
        sqlx::query("INSERT INTO as_users(user_id) VALUES (?) ON CONFLICT DO NOTHING")
            .bind(format!("@{localpart}:{}", config.server_name))
            .execute(&pool)
            .await?;
        pool.close().await;
        println!("User-scoped device session saved; credentials were not logged");
        return Ok(());
    }
    let mut router = m3_control::router();
    if let Some(path) = args.as_config {
        let config = m3_control::provision::Config::load(&path)?;
        let database = args
            .database
            .ok_or_else(|| anyhow::anyhow!("--database is required with --as-config"))?;
        let pool = m3_storage::open(&database, m3_storage::DatabaseKind::Control).await?;
        router = router.merge(m3_control::appservice::router(
            m3_control::appservice::Appservice {
                hs_token: Arc::from(config.hs_token),
                server_name: Arc::from(config.server_name),
                pool,
            },
        ));
    }
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();
    let listener = tokio::net::TcpListener::bind(args.listen).await?;
    tracing::info!(address = %listener.local_addr()?, "Control bootstrap listening; management runtime is not initialized");
    axum::serve(listener, router)
        .with_graceful_shutdown(async {
            if let Err(error) = shutdown_signal().await {
                tracing::error!(%error, "Failed to listen for shutdown signal");
            }
        })
        .await?;
    Ok(())
}

async fn shutdown_signal() -> std::io::Result<()> {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        tokio::select! {
            result = tokio::signal::ctrl_c() => result,
            _ = terminate.recv() => Ok(()),
        }
    }
    #[cfg(not(unix))]
    tokio::signal::ctrl_c().await
}
