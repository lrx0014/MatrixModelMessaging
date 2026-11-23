use std::time::Duration;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "m3", version, about = "M3 administrator CLI foundation")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Read Control process liveness. This does not establish runtime readiness.
    Health {
        #[arg(long, default_value = "http://127.0.0.1:8080")]
        control_url: reqwest::Url,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    match Args::parse().command {
        Command::Health { control_url } => {
            let url = control_url.join("/health/live")?;
            let body = reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .build()?
                .get(url)
                .send()
                .await?
                .error_for_status()?
                .text()
                .await?;
            println!("{body}");
        }
    }
    Ok(())
}
