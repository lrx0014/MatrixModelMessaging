//! Commands own only the named disposable M1 fixture and ignored local state.
use crate::{root, run};
use anyhow::{Context, ensure};
use std::{
    fs,
    path::PathBuf,
    process::{Child, Command},
    time::Duration,
};

#[derive(clap::Subcommand)]
pub enum Action {
    Up,
    Smoke,
    Images {
        #[arg(long)]
        arch: String,
    },
    Down,
}
fn directory() -> PathBuf {
    root().join(".local/m1")
}
fn compose(args: &[&str]) -> anyhow::Result<()> {
    run(Command::new("docker")
        .current_dir(root())
        .args([
            "compose",
            "--project-name",
            "m3-m1",
            "--file",
            "deploy/compose.m1.yaml",
        ])
        .args(args))
}
struct Receiver(Child);
impl Drop for Receiver {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

pub async fn execute(action: Action) -> anyhow::Result<()> {
    let dir = directory();
    match action {
        Action::Up => {
            ensure!(
                !dir.join("control.json").exists(),
                "Local M1 credentials already exist; use the existing fixture or explicitly archive it before creating another"
            );
            fs::create_dir_all(dir.join("sessions"))?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                for path in [&dir, &dir.join("sessions")] {
                    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
                }
            }
            compose(&["run", "--rm", "init"])?;
            compose(&[
                "up",
                "--detach",
                "--wait",
                "--wait-timeout",
                "60",
                "synapse",
                "element",
            ])?;
            run(Command::new("docker")
                .current_dir(root())
                .args([
                    "compose",
                    "--project-name",
                    "m3-m1",
                    "--file",
                    "deploy/compose.m1.yaml",
                    "cp",
                    "synapse:/data/control.json",
                ])
                .arg(dir.join("control.json")))?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(dir.join("control.json"), fs::Permissions::from_mode(0o600))?;
            }
        }
        Action::Down => compose(&["down", "--volumes"])?,
        Action::Images { arch } => {
            ensure!(
                matches!(arch.as_str(), "arm64" | "amd64"),
                "Expected arm64 or amd64"
            );
            run(Command::new("python3")
                .current_dir(root())
                .arg("tests/integration/matrix_images.py")
                .args(["--arch", &arch]))?;
        }
        Action::Smoke => {
            run(Command::new("cargo").current_dir(root()).args([
                "build",
                "--locked",
                "-p",
                "m3-agent",
                "-p",
                "m3-control",
            ]))?;
            let config = m3_control::provision::Config::load(&dir.join("control.json"))?;
            let pool = m3_storage::open(&dir.join("control.db"), m3_storage::DatabaseKind::Control)
                .await?;
            for name in ["alice", "bob"] {
                let session = dir.join("sessions").join(format!("{name}.json"));
                if !session.exists() {
                    m3_control::provision::provision(&config, &format!("_m3_{name}"), &session)
                        .await?;
                }
                sqlx::query("INSERT INTO as_users(user_id) VALUES (?) ON CONFLICT DO NOTHING")
                    .bind(format!("@_m3_{name}:{}", config.server_name))
                    .execute(&pool)
                    .await?;
            }
            drop(std::net::TcpListener::bind("0.0.0.0:18019").context(
                "M1 smoke must own the AS receiver port; stop an existing receiver first",
            )?);
            let log = fs::File::create(dir.join("control.log"))?;
            let _receiver = Receiver(
                Command::new(root().join("target/debug/m3-control"))
                    .args(["--listen", "0.0.0.0:18019", "--as-config"])
                    .arg(dir.join("control.json"))
                    .arg("--database")
                    .arg(dir.join("control.db"))
                    .stdout(log.try_clone()?)
                    .stderr(log)
                    .spawn()?,
            );
            let http = reqwest::Client::new();
            for attempt in 0..30 {
                if http
                    .post("http://127.0.0.1:18019/_matrix/app/v1/ping")
                    .bearer_auth(&config.hs_token)
                    .json(&serde_json::json!({}))
                    .send()
                    .await
                    .is_ok_and(|r| r.status().is_success())
                {
                    break;
                }
                ensure!(attempt < 29, "AS receiver did not start");
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            let ping = http
                .post(format!(
                    "{}/_matrix/client/v1/appservice/m3-m1/ping",
                    config.homeserver_url
                ))
                .bearer_auth(&config.as_token)
                .json(&serde_json::json!({"transaction_id":"m1-probe"}))
                .send()
                .await?;
            ensure!(
                ping.status().is_success(),
                "Real homeserver AS ping failed: HTTP {}",
                ping.status()
            );
            for name in ["alice", "bob"] {
                let session: serde_json::Value = serde_json::from_slice(&fs::read(
                    dir.join("sessions").join(format!("{name}.json")),
                )?)?;
                let token = session["access_token"]
                    .as_str()
                    .context("Missing user token")?;
                let own = http
                    .get(format!(
                        "{}/_matrix/client/v3/account/whoami?user_id=%40_m3_outside%3Am3.test",
                        config.homeserver_url
                    ))
                    .bearer_auth(token)
                    .send()
                    .await?;
                if own.status().is_success() {
                    ensure!(
                        own.json::<serde_json::Value>().await?["user_id"] == session["user_id"],
                        "User token impersonated another identity"
                    );
                } else {
                    ensure!(
                        own.status().as_u16() == 403,
                        "Unexpected identity-assertion response"
                    );
                }
                let denied=http.post(format!("{}/_matrix/client/v3/login",config.homeserver_url)).bearer_auth(token).json(&serde_json::json!({"type":"m.login.application_service","identifier":{"type":"m.id.user","user":"_m3_outside"}})).send().await?;
                ensure!(
                    denied.status().is_client_error(),
                    "User token retained AS provisioning authority"
                );
            }
            let outside=http.post(format!("{}/_matrix/client/v3/register",config.homeserver_url)).bearer_auth(&config.as_token).json(&serde_json::json!({"type":"m.login.application_service","username":"outside_namespace","inhibit_login":true})).send().await?;
            ensure!(
                outside.status().as_u16() == 400,
                "Homeserver did not enforce the AS namespace"
            );
            for (path, token, status) in [
                (
                    "/_matrix/app/v1/users/%40_m3_alice%3Am3.test",
                    config.hs_token.as_str(),
                    200,
                ),
                (
                    "/_matrix/app/v1/users/%40_m3_unknown%3Am3.test",
                    config.hs_token.as_str(),
                    404,
                ),
                (
                    "/_matrix/app/v1/users/%40_m3_alice%3Am3.test",
                    "invalid-fixture-token",
                    401,
                ),
            ] {
                let response = http
                    .get(format!("http://127.0.0.1:18019{path}"))
                    .bearer_auth(token)
                    .send()
                    .await?;
                ensure!(
                    response.status().as_u16() == status,
                    "AS query/authentication returned an unexpected status"
                );
            }
            for _ in 0..2 {
                let response = http
                    .put("http://127.0.0.1:18019/_matrix/app/v1/transactions/m1-redelivery-probe")
                    .bearer_auth(&config.hs_token)
                    .json(&serde_json::json!({"events":[]}))
                    .send()
                    .await?;
                ensure!(
                    response.status().is_success(),
                    "AS transaction acknowledgement failed"
                );
            }
            let copies: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM as_transactions WHERE transaction_id='m1-redelivery-probe'",
            )
            .fetch_one(&pool)
            .await?;
            ensure!(
                copies == 1,
                "AS redelivery duplicated durable acknowledgement"
            );
            let previous: i64 = sqlx::query_scalar("SELECT count(*) FROM as_transactions")
                .fetch_one(&pool)
                .await?;
            println!("Real Synapse AS ping and user-token scope boundaries passed");
            run(Command::new(root().join("target/debug/m3-agent"))
                .args([
                    "protocol-smoke",
                    "--homeserver",
                    &config.homeserver_url,
                    "--sessions",
                ])
                .arg(dir.join("sessions"))
                .arg("--state")
                .arg(dir.join("agents")))?;
            let mut delivered = false;
            for _ in 0..600 {
                let count: i64 = sqlx::query_scalar("SELECT count(*) FROM as_transactions")
                    .fetch_one(&pool)
                    .await?;
                if count > previous {
                    delivered = true;
                    break;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            ensure!(delivered, "No new real AS transactions were acknowledged");
            println!("Real AS transaction acknowledgements recorded without social processing");
            pool.close().await;
        }
    }
    Ok(())
}
