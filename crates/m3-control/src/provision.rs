//! Control-only AS credentials and standard user-scoped device provisioning.
use anyhow::{Context, ensure};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

#[derive(Deserialize)]
pub struct Config {
    pub homeserver_url: String,
    pub server_name: String,
    pub as_token: String,
    pub hs_token: String,
}
impl Config {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        serde_json::from_slice(&fs::read(path)?).context("Invalid Control configuration")
    }
}

/// Exclusive creation prevents accidentally overwriting an existing device
/// session. Partial provisioning is retained for explicit recovery (M2).
pub async fn provision(config: &Config, localpart: &str, output: &Path) -> anyhow::Result<()> {
    ensure!(
        localpart
            .strip_prefix("_m3_")
            .is_some_and(|id| !id.is_empty()
                && id
                    .bytes()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')),
        "User is outside the configured namespace"
    );
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(output)
        .context("Session output must not already exist")?;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    let base = format!(
        "{}/_matrix/client/v3",
        config.homeserver_url.trim_end_matches('/')
    );
    let response=client.post(format!("{base}/register")).bearer_auth(&config.as_token)
        .json(&json!({"type":"m.login.application_service","username":localpart,"inhibit_login":true})).send().await?;
    // Do not expose response bodies: homeserver responses can contain credentials.
    ensure!(
        response.status().is_success(),
        "AS registration failed: HTTP {} (partial state retained)",
        response.status()
    );
    let response=client.post(format!("{base}/login")).bearer_auth(&config.as_token)
        .json(&json!({"type":"m.login.application_service","identifier":{"type":"m.id.user","user":localpart},"initial_device_display_name":"M3 protocol prototype"})).send().await?;
    ensure!(
        response.status().is_success(),
        "User-scoped AS login failed: HTTP {}",
        response.status()
    );
    let session: Value = response.json().await?;
    let expected = format!("@{localpart}:{}", config.server_name);
    ensure!(
        session["user_id"].as_str() == Some(expected.as_str()),
        "AS login returned another identity"
    );
    ensure!(
        session["access_token"]
            .as_str()
            .is_some_and(|t| !t.is_empty() && t != config.as_token),
        "AS login did not return an independent token"
    );
    ensure!(
        session["device_id"].as_str().is_some_and(|d| !d.is_empty()),
        "AS login returned no device"
    );
    let who = client
        .get(format!("{base}/account/whoami"))
        .bearer_auth(session["access_token"].as_str().unwrap())
        .send()
        .await?;
    ensure!(who.status().is_success(), "Device token was rejected");
    let who: Value = who.json().await?;
    ensure!(
        who["user_id"] == session["user_id"]
            && who["device_id"] == session["device_id"]
            && who["is_guest"] == false,
        "Device token identity mismatch"
    );
    file.write_all(&serde_json::to_vec(&session)?)?;
    file.sync_all()?;
    Ok(())
}
