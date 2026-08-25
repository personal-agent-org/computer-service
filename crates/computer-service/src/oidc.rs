//! One-time ownership verification for Computer Service using the shared OAuth device grant.
//! The temporary user tokens are exchanged for a device-bound service token and never persisted.
//! The CLI prints a URL + code, the user authorizes in a browser, and it polls for a short-lived
//! access token without requesting `offline_access`. That token is immediately exchanged for a
//! device-bound Computer Service credential and is never used by the running service.
//!
//! Works with external OIDC and with the backend's local identity provider:
//! the endpoints come from the server's client-config, not from the issuer's URL shape.

use anyhow::Result;
use pa_oidc::{Endpoints, Prompt, Tokens};

struct AgentPrompt;

impl Prompt for AgentPrompt {
    fn authorize(&self, url: &str, user_code: &str) {
        eprintln!("{}", crate::i18n::authorization(url, user_code));
    }

    fn failed(&self, error: &str) -> String {
        crate::i18n::authorization_failed(error)
    }
}

/// Run the full device-code login flow; blocks until the user authorizes (or it errors).
pub async fn device_login(endpoints: &Endpoints, client_id: &str) -> Result<Tokens> {
    pa_oidc::device_login(endpoints, client_id, &AgentPrompt).await
}
