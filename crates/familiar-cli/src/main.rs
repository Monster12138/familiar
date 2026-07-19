pub mod hook_reporter;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Hook subcommand
    Hook,
    /// Status subcommand
    Status,
    /// Headless subcommand
    Headless,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Hook => {
            println!("Running hook subcommand");
            // Example usage of hook_reporter:
            // hook_reporter::forward_stdin_to_socket("/tmp/familiar.sock").await?;
        }
        Commands::Status => {
            println!("Running status subcommand");
        }
        Commands::Headless => {
            println!("Running headless subcommand");
        }
    }

    Ok(())
}
