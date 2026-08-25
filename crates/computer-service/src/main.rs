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
    Enroll {
        #[arg(long)]
        server: String,
        #[arg(long)]
        device: String,
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
        Command::Enroll {
            server,
            device,
            workspace,
        } => computer_service::enroll(server, device, workspace).await,
        Command::Run => computer_service::run().await,
        Command::Tools => computer_service::tools().await,
        Command::CredentialHelper { operation } => {
            computer_service::credential_helper(&operation).await
        }
    }
}
