use clap::{Parser, Subcommand};

/// cntl — a Rust-based VCS with content-addressable storage
/// and a two-tier history model.
#[derive(Parser)]
#[command(name = "cntl", version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Initialize a new cntl repository in the current directory
    Init,

    /// Get or set configuration values
    Config {
        /// Configuration key (e.g., user.name)
        key: String,

        /// Configuration value (omit to read the current value)
        value: Option<String>,

        /// Write to repository-local config instead of global
        #[arg(long)]
        local: bool,
    },

    /// Show changes between the working tree and HEAD
    Status,

    /// Record a new commit with all current changes
    Commit {
        /// Commit message
        #[arg(short, long)]
        message: String,
    },

    /// Show commit history
    Log,
}

pub fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Init => {
            println!("init: not implemented yet");
        }
        Command::Config { key, value, local } => {
            println!(
                "config: key={key} value={value:?} local={local} — not implemented yet"
            );
        }
        Command::Status => {
            println!("status: not implemented yet");
        }
        Command::Commit { message } => {
            println!("commit: message={message:?} — not implemented yet");
        }
        Command::Log => {
            println!("log: not implemented yet");
        }
    }

    Ok(())
}
