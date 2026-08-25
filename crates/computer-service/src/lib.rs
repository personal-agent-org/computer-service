//! Personal Agent Computer Service. It connects a computer to Personal Agent and exposes
//! explicitly enabled tools and capabilities. It never stores a user/chat credential.

mod client;
mod config;
mod credential;
mod i18n;
mod jail;
mod lsp;
mod oidc;
mod proc;
mod pty;
mod sandbox;

use anyhow::Result;

use config::Config;

#[derive(serde::Deserialize)]
struct Enrollment {
    token: String,
}

async fn exchange_for_service_token(
    server: &str,
    device: &str,
    user_access_token: &str,
) -> Result<String> {
    let url = format!(
        "{}/api/v1/devices/{}/computer-service/enroll",
        server.trim_end_matches('/'),
        device
    );
    let response = pa_oidc::tls::http_client()
        .post(url)
        .bearer_auth(user_access_token)
        .send()
        .await?;
    if !response.status().is_success() {
        anyhow::bail!(i18n::enrollment_failed(response.status()));
    }
    let enrollment: Enrollment = response.json().await?;
    Ok(enrollment.token)
}

/// Log in via the discovered device flow (external OIDC or backend-local auth), exchange the
/// short-lived user credential for a dedicated service token, and store only that token.
pub async fn enroll(server: String, device: String, workspace: String) -> Result<()> {
    let abs = std::fs::canonicalize(&workspace)
        .map(|p| p.display().to_string())
        .unwrap_or(workspace);
    let disco = pa_oidc::discover(&server).await?;
    let client = disco.device_client_id;
    // The user token exists only long enough to prove ownership and exchange it for a dedicated,
    // device-bound Computer Service token. Neither access nor refresh token is persisted.
    let user_tokens = oidc::device_login(&disco.endpoints, &client).await?;
    let service_token =
        exchange_for_service_token(&server, &device, &user_tokens.access_token).await?;
    let cfg = Config {
        server,
        device_id: device,
        workspace: abs,
        service_token,
        sandbox_token: None,
        home_root: None, // default $HOME at runtime via home_root_resolved()
        sandbox: false,  // opt-in OS command sandbox; user enables in config.toml
        disabled_tools: Vec::new(), // everything exposed until the desktop app narrows it
        expose_home_index: true,
        jail: true, // jailed by default; user sets `jail = false` in config.toml
    };
    config::save(&cfg)?;
    println!(
        "{}",
        i18n::connected(
            &cfg.device_id,
            &config::config_path().display().to_string(),
            &cfg.workspace,
        )
    );
    println!("{}", i18n::start_hint());
    Ok(())
}

/// Connect and serve tool calls.
pub async fn run() -> Result<()> {
    // A backend-spawned cloud sandbox injects its config via env (PA_SANDBOX_TOKEN);
    // a normal device reads the enrolled config file.
    let cfg = match Config::from_env() {
        Some(c) => c,
        None => config::load()?,
    };
    client::run(cfg).await
}

/// Print the available coding tools (name + description) as JSON. The desktop app uses this
/// to render the per-device tool-exposure toggles.
pub async fn tools() -> Result<()> {
    println!("{}", jail::tool_specs());
    Ok(())
}

/// Git credential helper (invoked by git for Personal-Agent-cloned repos). Internal.
pub async fn credential_helper(operation: &str) -> Result<()> {
    credential::run(operation).await
}
