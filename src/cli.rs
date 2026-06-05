use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use rusqlite::Connection;

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

pub fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Init => {
            init()?;
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

const SCHEMA_SQL: &str = "\
CREATE TABLE objects (
    hash     BLOB PRIMARY KEY,
    obj_type TEXT NOT NULL,
    data     BLOB NOT NULL
);
CREATE TABLE refs (
    name   TEXT PRIMARY KEY,
    target BLOB NOT NULL
);
CREATE TABLE settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);";

fn init() -> Result<()> {
    let cntl_dir = PathBuf::from(".cntl");
    if cntl_dir.exists() {
        bail!(".cntl already exists in the current directory");
    }

    fs::create_dir(&cntl_dir).context("failed to create .cntl directory")?;

    let db_path = cntl_dir.join("repo.db");
    let conn = Connection::open(&db_path)
        .with_context(|| format!("failed to create {}", db_path.display()))?;
    conn.execute_batch(SCHEMA_SQL)
        .context("failed to initialize repo.db schema")?;

    let shown = fs::canonicalize(&cntl_dir).unwrap_or(cntl_dir);
    println!("Initialized empty cntl repository in {}", shown.display());
    Ok(())
}
