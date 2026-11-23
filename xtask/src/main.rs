mod database;
mod matrix;

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command as Process,
};

use anyhow::{Context, ensure};
use clap::{Parser, Subcommand};
use utoipa::OpenApi;

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Export Rust OpenAPI, or reject a stale artifact.
    Openapi {
        #[arg(long)]
        check: bool,
    },
    /// Prepare SQLx metadata against fresh migrated databases, or check for drift.
    Sqlx {
        #[arg(long)]
        check: bool,
    },
    /// Export/check OpenAPI and generated TypeScript together (requires npm ci).
    Contracts {
        #[arg(long)]
        check: bool,
    },
    /// Exercise negative drift checks and verify they leave artifacts unchanged.
    DriftTest,
    /// Run the same deterministic checks used by CI (requires pinned Node/npm).
    Verify,
    /// Manage the separate local M1 protocol and Element fixture.
    Matrix {
        #[command(subcommand)]
        action: matrix::Action,
    },
    /// Manage only the disposable m3-m0 Compose project.
    Synapse {
        #[command(subcommand)]
        action: SynapseAction,
    },
}

#[derive(Subcommand)]
enum SynapseAction {
    Up,
    Smoke,
    Reset,
    Down,
}

fn compose(args: &[&str]) -> anyhow::Result<()> {
    run(Process::new("docker")
        .current_dir(root())
        .args([
            "compose",
            "--project-name",
            "m3-m0",
            "--file",
            "deploy/compose.test.yaml",
        ])
        .args(args))
}

fn synapse_up() -> anyhow::Result<()> {
    compose(&["run", "--rm", "init"])?;
    compose(&[
        "up",
        "--detach",
        "--wait",
        "--wait-timeout",
        "60",
        "synapse",
    ])
}

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

pub fn run(command: &mut Process) -> anyhow::Result<()> {
    let status = command
        .status()
        .with_context(|| format!("Failed to start {command:?}"))?;
    ensure!(status.success(), "Command failed: {command:?}");
    Ok(())
}

fn openapi(check: bool) -> anyhow::Result<()> {
    let path = root().join("api/openapi.json");
    let expected = format!("{}\n", m3_control::ApiDoc::openapi().to_pretty_json()?);
    if check {
        ensure!(
            fs::read_to_string(&path)? == expected,
            "OpenAPI is stale; run cargo xtask contracts"
        );
    } else {
        fs::write(path, expected)?;
    }
    Ok(())
}

fn types(check: bool) -> anyhow::Result<()> {
    let mut command = Process::new("node");
    command
        .current_dir(root().join("web"))
        .arg("scripts/generate.mjs");
    if check {
        command.arg("--check");
    }
    run(&mut command)
}

// Restore even when a checker fails, so the negative test is reversible.
struct Restore {
    path: PathBuf,
    original: Vec<u8>,
}
impl Drop for Restore {
    fn drop(&mut self) {
        if let Err(error) = fs::write(&self.path, &self.original) {
            eprintln!("Failed to restore {}: {error}", self.path.display());
        }
    }
}

async fn reject_mutation(path: &Path, checker: &str) -> anyhow::Result<()> {
    let original = fs::read(path)?;
    let restore = Restore {
        path: path.to_path_buf(),
        original,
    };
    let mutated = if checker == "sqlx" {
        let mut metadata: serde_json::Value = serde_json::from_slice(&restore.original)?;
        metadata["hash"] = serde_json::json!("intentionally-stale");
        serde_json::to_vec_pretty(&metadata)?
    } else {
        [restore.original.as_slice(), b"\n"].concat()
    };
    fs::write(path, mutated)?;
    let result = match checker {
        "openapi" => openapi(true),
        "types" => types(true),
        "sqlx" => database::prepare(true).await,
        _ => unreachable!(),
    };
    ensure!(
        result.is_err(),
        "{checker} checker accepted an intentionally stale artifact"
    );
    drop(restore);
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    match Args::parse().command {
        Command::Matrix { action } => matrix::execute(action).await?,
        Command::Openapi { check } => openapi(check)?,
        Command::Sqlx { check } => database::prepare(check).await?,
        Command::Contracts { check } => {
            openapi(check)?;
            types(check)?;
        }
        Command::Synapse { action } => match action {
            SynapseAction::Up => synapse_up()?,
            SynapseAction::Smoke => {
                compose(&["exec", "-T", "synapse", "python", "/fixtures/smoke.py"])?
            }
            SynapseAction::Reset => {
                compose(&["down", "--volumes"])?;
                synapse_up()?;
            }
            SynapseAction::Down => compose(&["down", "--volumes"])?,
        },
        Command::DriftTest => drift_test().await?,
        Command::Verify => {
            run(Process::new("cargo")
                .current_dir(root())
                .args(["fmt", "--all", "--", "--check"]))?;
            run(Process::new("cargo").current_dir(root()).args([
                "clippy",
                "--workspace",
                "--all-targets",
                "--locked",
                "--",
                "-D",
                "warnings",
            ]))?;
            run(Process::new("cargo").current_dir(root()).args([
                "test",
                "--workspace",
                "--locked",
            ]))?;
            run(Process::new("npm").current_dir(root()).args([
                "run",
                "typecheck",
                "--prefix",
                "web",
            ]))?;
            drift_test().await?;
        }
    }
    Ok(())
}

async fn drift_test() -> anyhow::Result<()> {
    openapi(true)?;
    types(true)?;
    database::prepare(true).await?;
    reject_mutation(&root().join("api/openapi.json"), "openapi").await?;
    reject_mutation(&root().join("web/src/generated/api.ts"), "types").await?;
    let mut metadata: Vec<_> = fs::read_dir(root().join(".sqlx"))?.collect::<Result<_, _>>()?;
    metadata.sort_by_key(|entry| entry.file_name());
    let path = metadata.first().context("No SQLx metadata to test")?.path();
    reject_mutation(&path, "sqlx").await?;
    openapi(true)?;
    types(true)?;
    database::prepare(true).await?;
    println!("OpenAPI, TypeScript, and SQLx reject stale artifacts and pass after restoration");
    Ok(())
}
