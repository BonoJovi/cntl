use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use directories::BaseDirs;
use rusqlite::{Connection, OptionalExtension, params};

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
            config(key, value, local)?;
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

const SETTINGS_SCHEMA_SQL: &str = "\
CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);";

fn config(key: String, value: Option<String>, local: bool) -> Result<()> {
    match value {
        Some(v) => write_config(&key, &v, local),
        None => read_config(&key, local),
    }
}

fn write_config(key: &str, value: &str, local: bool) -> Result<()> {
    let path = if local {
        local_db_path_existing()?
    } else {
        let path = global_db_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        path
    };

    let conn = Connection::open(&path)
        .with_context(|| format!("failed to open {}", path.display()))?;
    conn.execute_batch(SETTINGS_SCHEMA_SQL)
        .context("failed to ensure settings table")?;
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )
    .context("failed to write setting")?;
    Ok(())
}

fn read_config(key: &str, local: bool) -> Result<()> {
    let value = if local {
        let path = local_db_path_existing()?;
        read_setting(&path, key)?
    } else {
        // local → global → error
        let local_path = PathBuf::from(".cntl/repo.db");
        let local_val = if local_path.exists() {
            read_setting(&local_path, key)?
        } else {
            None
        };
        match local_val {
            Some(v) => Some(v),
            None => {
                let global_path = global_db_path()?;
                read_setting(&global_path, key)?
            }
        }
    };

    match value {
        Some(v) => {
            println!("{v}");
            Ok(())
        }
        None => bail!("config key not set: {key}"),
    }
}

fn read_setting(path: &Path, key: &str) -> Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }
    let conn = Connection::open(path)
        .with_context(|| format!("failed to open {}", path.display()))?;
    let value: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .optional()
        .context("failed to read setting")?;
    Ok(value)
}

fn local_db_path_existing() -> Result<PathBuf> {
    let path = PathBuf::from(".cntl/repo.db");
    if !path.exists() {
        bail!("not in a cntl repository (no .cntl in current directory)");
    }
    Ok(path)
}

fn global_db_path() -> Result<PathBuf> {
    if let Some(home) = std::env::var_os("CNTL_HOME") {
        return Ok(PathBuf::from(home).join("global.db"));
    }
    let base = BaseDirs::new().context("could not determine user config directory")?;
    Ok(base.config_dir().join("cntl").join("global.db"))
}
