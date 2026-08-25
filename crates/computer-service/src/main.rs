//! Connect this computer to Personal Agent and expose explicitly enabled capabilities.
//! This service is a capability provider, not a chat client.

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "pacs",
    version,
    about = "Expose this computer's capabilities to Personal Agent"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Verify ownership using the device flow and obtain a device-bound service token.
    ///
    /// The server URL is all it needs: this machine registers itself, or renews the device it
    /// is already enrolled as (personal-agent-org/personal-agent#122).
    Enroll {
        #[arg(long)]
        server: String,
        #[arg(long, default_value = ".")]
        workspace: String,
    },
    /// Connect and serve the enabled capabilities.
    #[command(alias = "start")]
    Run,
    /// Print the available tool catalog as JSON.
    Tools,
    /// Git credential helper used for repositories cloned by Personal Agent.
    #[command(hide = true)]
    CredentialHelper {
        #[arg(default_value = "get")]
        operation: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    match Cli::parse().cmd {
        Command::Enroll { server, workspace } => computer_service::enroll(server, workspace).await,
        Command::Run => computer_service::run().await,
        Command::Tools => computer_service::tools().await,
        Command::CredentialHelper { operation } => {
            computer_service::credential_helper(&operation).await
        }
    }
}
