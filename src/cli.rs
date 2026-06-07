use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use directories::BaseDirs;
use rusqlite::{Connection, OptionalExtension, params};

use crate::object::{self, Blob, Commit, ObjectHash, Tree, TreeEntry, TreeEntryKind};

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
            status()?;
        }
        Command::Commit { message } => {
            commit(&message)?;
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
        lookup_config(key)?
    };

    match value {
        Some(v) => {
            println!("{v}");
            Ok(())
        }
        None => bail!("config key not set: {key}"),
    }
}

/// Resolve a config key with the standard local → global precedence.
fn lookup_config(key: &str) -> Result<Option<String>> {
    let local_path = PathBuf::from(".cntl/repo.db");
    if local_path.exists() {
        if let Some(v) = read_setting(&local_path, key)? {
            return Ok(Some(v));
        }
    }
    let global_path = global_db_path()?;
    read_setting(&global_path, key)
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

fn status() -> Result<()> {
    let repo_db = local_db_path_existing()?;

    let conn = Connection::open(&repo_db)
        .with_context(|| format!("failed to open {}", repo_db.display()))?;
    let head: Option<Vec<u8>> = conn
        .query_row(
            "SELECT target FROM refs WHERE name = 'HEAD'",
            [],
            |row| row.get(0),
        )
        .optional()
        .context("failed to read HEAD")?;

    let files = scan_working_tree(Path::new("."))?;

    if head.is_none() {
        println!("No commits yet.");
        println!();
        if files.is_empty() {
            println!("nothing to commit (working tree empty)");
        } else {
            println!("Untracked files:");
            for path in &files {
                println!("\tnew file:   {path}");
            }
        }
    } else {
        // HEAD tree comparison lands together with `cntl commit`.
        println!("HEAD exists, but tree comparison is not implemented yet.");
    }

    Ok(())
}

fn scan_working_tree(root: &Path) -> Result<Vec<String>> {
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| e.file_name() != ".cntl")
    {
        let entry = entry.context("failed to walk working tree")?;
        if !entry.file_type().is_file() {
            continue;
        }
        let rel = entry.path().strip_prefix(root).unwrap_or(entry.path());
        files.push(rel.to_string_lossy().into_owned());
    }
    files.sort();
    Ok(files)
}

fn commit(message: &str) -> Result<()> {
    let repo_db = local_db_path_existing()?;

    let author_name = lookup_config("user.name")?.ok_or_else(|| {
        anyhow::anyhow!("user.name not configured (run `cntl config user.name \"...\"`)")
    })?;
    let author_email = lookup_config("user.email")?.ok_or_else(|| {
        anyhow::anyhow!("user.email not configured (run `cntl config user.email \"...\"`)")
    })?;

    let mut conn = Connection::open(&repo_db)
        .with_context(|| format!("failed to open {}", repo_db.display()))?;
    let tx = conn.transaction().context("failed to begin transaction")?;

    let tree_hash = write_tree_recursive(&tx, Path::new("."))?;

    let parent: Option<ObjectHash> = tx
        .query_row(
            "SELECT target FROM refs WHERE name = 'HEAD'",
            [],
            |row| {
                let bytes: Vec<u8> = row.get(0)?;
                bytes.try_into().map_err(|_| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Blob,
                        "HEAD target is not a 32-byte hash".into(),
                    )
                })
            },
        )
        .optional()
        .context("failed to read HEAD")?;

    let commit_obj = Commit {
        parent,
        tree: tree_hash,
        author_name,
        author_email,
        timestamp_utc: chrono::Utc::now().timestamp(),
        message: message.to_string(),
    };
    let commit_bytes = object::encode(&commit_obj)?;
    let commit_hash = object::hash_bytes(&commit_bytes);
    object::store_object(&tx, &commit_hash, "commit", &commit_bytes)?;

    tx.execute(
        "INSERT INTO refs (name, target) VALUES ('HEAD', ?1)
         ON CONFLICT(name) DO UPDATE SET target = excluded.target",
        params![&commit_hash[..]],
    )
    .context("failed to update HEAD")?;

    tx.commit().context("failed to commit transaction")?;

    println!("[{}] {message}", hex_short(&commit_hash));
    Ok(())
}

fn write_tree_recursive(conn: &Connection, dir: &Path) -> Result<ObjectHash> {
    let mut children: Vec<_> = fs::read_dir(dir)
        .with_context(|| format!("failed to read directory {}", dir.display()))?
        .collect::<std::io::Result<Vec<_>>>()
        .context("failed to enumerate directory")?;
    children.sort_by_key(|e| e.file_name());

    let mut entries: Vec<TreeEntry> = Vec::new();
    for child in children {
        let name = child.file_name().to_string_lossy().into_owned();
        if name == ".cntl" {
            continue;
        }
        let path = child.path();
        let file_type = child
            .file_type()
            .with_context(|| format!("failed to read file type of {}", path.display()))?;

        if file_type.is_file() {
            let data = fs::read(&path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            let blob = Blob { data };
            let bytes = object::encode(&blob)?;
            let hash = object::hash_bytes(&bytes);
            object::store_object(conn, &hash, "blob", &bytes)?;
            entries.push(TreeEntry {
                name,
                kind: TreeEntryKind::Blob,
                hash,
            });
        } else if file_type.is_dir() {
            let subtree = write_tree_recursive(conn, &path)?;
            entries.push(TreeEntry {
                name,
                kind: TreeEntryKind::Tree,
                hash: subtree,
            });
        }
    }

    let tree = Tree { entries };
    let bytes = object::encode(&tree)?;
    let hash = object::hash_bytes(&bytes);
    object::store_object(conn, &hash, "tree", &bytes)?;
    Ok(hash)
}

fn hex_short(hash: &ObjectHash) -> String {
    let mut s = String::with_capacity(8);
    for b in hash.iter().take(4) {
        use std::fmt::Write;
        let _ = write!(s, "{b:02x}");
    }
    s
}

fn global_db_path() -> Result<PathBuf> {
    if let Some(home) = std::env::var_os("CNTL_HOME") {
        return Ok(PathBuf::from(home).join("global.db"));
    }
    let base = BaseDirs::new().context("could not determine user config directory")?;
    Ok(base.config_dir().join("cntl").join("global.db"))
}
