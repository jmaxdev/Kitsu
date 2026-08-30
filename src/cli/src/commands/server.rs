use anyhow::Result;
use colored::*;
use kitsu_core::server::{
    ServerToken, ensure_server_started, is_server_running, run_server, stop_server,
};

use crate::app::ServerAction;

/// Executes the `server` subcommand.
pub fn execute(action: ServerAction) -> Result<()> {
    match action {
        ServerAction::Start { port } => {
            if is_server_running(port) {
                println!(
                    "{} Kitsu server is already running on port {}.",
                    "✓".green().bold(),
                    port.to_string().cyan()
                );
            } else {
                ensure_server_started()?;
                if is_server_running(port) {
                    println!(
                        "{} Kitsu server started successfully on http://127.0.0.1:{}",
                        "✓".green().bold(),
                        port.to_string().cyan()
                    );
                    let token = ServerToken::read()?;
                    println!("  API Bearer Token: {}", token.bright_black());
                } else {
                    println!("{}", "Failed to start Kitsu server.".red());
                }
            }
        }
        ServerAction::Off { port } => {
            if !is_server_running(port) {
                println!("Kitsu server is not running.");
            } else {
                let stopped = stop_server(port)?;
                if stopped {
                    println!(
                        "{} Kitsu server on port {} has been stopped.",
                        "✓".green().bold(),
                        port.to_string().cyan()
                    );
                } else {
                    println!(
                        "{} Sent shutdown signal to Kitsu server.",
                        "✓".green().bold()
                    );
                }
            }
        }
        ServerAction::Status { port } => {
            if is_server_running(port) {
                println!("{}", "=== Kitsu Server Status ===".cyan().bold());
                println!("  Status:   {}", "RUNNING".green().bold());
                println!("  Endpoint: http://127.0.0.1:{}", port.to_string().yellow());
                let token = ServerToken::read()?;
                println!("  Token:    {}", token.bright_black());
                println!("  API Base: http://127.0.0.1:{}/api/v1/", port);
            } else {
                println!("{}", "=== Kitsu Server Status ===".cyan().bold());
                println!("  Status:   {}", "STOPPED".red().bold());
                println!("  Port:     {}", port.to_string().yellow());
                println!("  Use 'kitsu server start' to launch the background daemon.");
            }
        }
        ServerAction::Token => {
            let token = ServerToken::get_or_create()?;
            println!("{}", token);
        }
        ServerAction::Daemon { port } => {
            run_server(port)?;
        }
    }
    Ok(())
}
