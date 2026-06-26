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
        /// Configuration key (e.g., user.name). Omit when using --all.
        key: Option<String>,

        /// Configuration value (omit to read the current value)
        value: Option<String>,

        /// Write to repository-local config instead of global
        #[arg(long)]
        local: bool,

        /// List all active configuration values with their scope
        #[arg(long, conflicts_with_all = ["key", "value", "local"])]
        all: bool,

        /// With --all, also show values shadowed by higher-priority scopes
        #[arg(long, requires = "all")]
        verbose: bool,
    },

    /// Show changes between the working tree and HEAD
    Status,

    /// Show unified diff between the working tree and HEAD
    Diff,

    /// Restore working-tree files from HEAD
    Restore {
        /// Paths to restore (file paths only; directories not yet supported)
        #[arg(required = true)]
        paths: Vec<String>,
    },

    /// Record a new commit with all current changes
    Commit {
        /// Commit message
        #[arg(short, long)]
        message: String,
    },

    /// Show commit history
    Log,

    /// List, create, or delete branches
    Branch {
        /// Branch name (omit to list all branches)
        name: Option<String>,

        /// Delete the named branch (cannot be the current branch)
        #[arg(short = 'd', long = "delete", requires = "name")]
        delete: bool,
    },

    /// Switch to another branch, updating the working tree to match
    Checkout {
        /// Branch to switch to
        name: String,
    },
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Init => {
            init()?;
        }
        Command::Config {
            key,
            value,
            local,
            all,
            verbose,
        } => {
            config(key, value, local, all, verbose)?;
        }
        Command::Status => {
            status()?;
        }
        Command::Diff => {
            diff()?;
        }
        Command::Restore { paths } => {
            restore(paths)?;
        }
        Command::Commit { message } => {
            commit(&message)?;
        }
        Command::Log => {
            log()?;
        }
        Command::Branch { name, delete } => {
            branch(name, delete)?;
        }
        Command::Checkout { name } => {
            checkout(&name)?;
        }
    }

    Ok(())
}

/// Schema version stored in `settings`. Bumped whenever the on-disk shape
/// changes in a way older versions of cntl cannot read.
const SCHEMA_VERSION: &str = "2";

/// Default branch name created at the first commit of a fresh repository.
const DEFAULT_BRANCH: &str = "main";

/// All branch refs live under this prefix in the `refs` table.
const BRANCH_REF_PREFIX: &str = "refs/heads/";

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
CREATE TABLE head (
    id       INTEGER PRIMARY KEY CHECK (id = 1),
    ref_name TEXT NOT NULL
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

    let default_ref = format!("{BRANCH_REF_PREFIX}{DEFAULT_BRANCH}");
    conn.execute(
        "INSERT INTO head (id, ref_name) VALUES (1, ?1)",
        params![default_ref],
    )
    .context("failed to initialize HEAD")?;
    conn.execute(
        "INSERT INTO settings (key, value) VALUES ('schema_version', ?1)",
        params![SCHEMA_VERSION],
    )
    .context("failed to record schema version")?;

    let shown = fs::canonicalize(&cntl_dir).unwrap_or(cntl_dir);
    println!("Initialized empty cntl repository in {}", shown.display());
    Ok(())
}

/// Open the repository DB and bring it up to the current schema if needed.
///
/// v0.1.0 stored `HEAD` as a row in `refs` whose `target` was a commit hash.
/// v0.2.0 introduces a dedicated `head` table holding a symbolic ref name,
/// and rewrites the legacy `HEAD` row as `refs/heads/main`.
fn open_repo(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)
        .with_context(|| format!("failed to open {}", path.display()))?;
    migrate_to_v2(&conn)?;
    Ok(conn)
}

fn migrate_to_v2(conn: &Connection) -> Result<()> {
    let has_head_table: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='head')",
            [],
            |row| row.get(0),
        )
        .context("failed to inspect schema")?;
    if has_head_table {
        return Ok(());
    }

    let tx_conn = conn;
    tx_conn.execute_batch(
        "CREATE TABLE head (
            id       INTEGER PRIMARY KEY CHECK (id = 1),
            ref_name TEXT NOT NULL
        );",
    )
    .context("failed to create head table")?;

    let default_ref = format!("{BRANCH_REF_PREFIX}{DEFAULT_BRANCH}");
    let legacy_head: Option<Vec<u8>> = tx_conn
        .query_row(
            "SELECT target FROM refs WHERE name = 'HEAD'",
            [],
            |row| row.get(0),
        )
        .optional()
        .context("failed to read legacy HEAD")?;
    if let Some(commit_hash) = legacy_head {
        tx_conn
            .execute("DELETE FROM refs WHERE name = 'HEAD'", [])
            .context("failed to delete legacy HEAD row")?;
        tx_conn
            .execute(
                "INSERT INTO refs (name, target) VALUES (?1, ?2)",
                params![default_ref, commit_hash],
            )
            .context("failed to install migrated branch ref")?;
    }
    tx_conn
        .execute(
            "INSERT INTO head (id, ref_name) VALUES (1, ?1)",
            params![default_ref],
        )
        .context("failed to install HEAD row")?;
    tx_conn
        .execute(
            "INSERT INTO settings (key, value) VALUES ('schema_version', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![SCHEMA_VERSION],
        )
        .context("failed to record schema version")?;
    Ok(())
}

const SETTINGS_SCHEMA_SQL: &str = "\
CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);";

fn config(
    key: Option<String>,
    value: Option<String>,
    local: bool,
    all: bool,
    verbose: bool,
) -> Result<()> {
    if verbose && !all {
        bail!("--verbose can only be used together with --all");
    }
    if all {
        return list_config(verbose);
    }
    let key = key.ok_or_else(|| anyhow::anyhow!("missing config key (or pass --all)"))?;
    match value {
        Some(v) => write_config(&key, &v, local),
        None => read_config(&key, local),
    }
}

fn list_config(verbose: bool) -> Result<()> {
    let local_path = PathBuf::from(".cntl/repo.db");
    let local_entries = if local_path.exists() {
        read_all_settings(&local_path)?
    } else {
        Vec::new()
    };
    let global_path = global_db_path()?;
    let global_entries = if global_path.exists() {
        read_all_settings(&global_path)?
    } else {
        Vec::new()
    };

    let local_map: std::collections::BTreeMap<String, String> =
        local_entries.into_iter().collect();
    let global_map: std::collections::BTreeMap<String, String> =
        global_entries.into_iter().collect();

    let mut all_keys: std::collections::BTreeSet<&String> = std::collections::BTreeSet::new();
    all_keys.extend(local_map.keys());
    all_keys.extend(global_map.keys());

    if all_keys.is_empty() {
        println!("no configuration set");
        return Ok(());
    }

    for key in all_keys {
        let (effective_scope, effective_value, shadowed) = match (local_map.get(key), global_map.get(key)) {
            (Some(v), Some(g)) => ("local", v.as_str(), Some(g.as_str())),
            (Some(v), None) => ("local", v.as_str(), None),
            (None, Some(g)) => ("global", g.as_str(), None),
            (None, None) => unreachable!("key came from one of the maps"),
        };
        println!("{key}({effective_scope}) \"{effective_value}\"");
        if verbose {
            if let Some(g) = shadowed {
                println!("  shadowed: global = \"{g}\"");
            }
        }
    }
    Ok(())
}

fn read_all_settings(path: &Path) -> Result<Vec<(String, String)>> {
    let conn = Connection::open(path)
        .with_context(|| format!("failed to open {}", path.display()))?;
    let mut stmt = conn
        .prepare("SELECT key, value FROM settings")
        .context("failed to prepare settings query")?;
    let rows = stmt
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
        .context("failed to query settings")?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.context("failed to read setting row")?);
    }
    Ok(out)
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
    let conn = open_repo(&repo_db)?;
    let head = read_head(&conn)?;
    let files = scan_working_tree(Path::new("."))?;

    let Some(head_hash) = head else {
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
        return Ok(());
    };

    let changes = compute_working_changes(&conn, Some(&head_hash), &files)?;
    if changes.is_clean() {
        println!("nothing to commit, working tree clean");
        return Ok(());
    }

    println!("Changes since last commit:");
    for path in &changes.modified {
        println!("\tmodified:   {path}");
    }
    for path in &changes.new_files {
        println!("\tnew file:   {path}");
    }
    for path in &changes.deleted {
        println!("\tdeleted:    {path}");
    }
    Ok(())
}

/// Difference between the working tree and a commit's tree, classified the
/// same way `cntl status` reports it.
struct WorkingChanges {
    modified: Vec<String>,
    new_files: Vec<String>,
    deleted: Vec<String>,
}

impl WorkingChanges {
    fn is_clean(&self) -> bool {
        self.modified.is_empty() && self.new_files.is_empty() && self.deleted.is_empty()
    }
}

/// Compare the scanned working-tree `files` against the tree of `head`
/// (or the empty tree when `head` is `None`, i.e. before the first commit).
fn compute_working_changes(
    conn: &Connection,
    head: Option<&ObjectHash>,
    files: &[String],
) -> Result<WorkingChanges> {
    let mut head_files: std::collections::HashMap<String, ObjectHash> =
        std::collections::HashMap::new();
    if let Some(head_hash) = head {
        let head_commit = object::load_commit(conn, head_hash)?;
        collect_tree_files(conn, &head_commit.tree, "", &mut head_files)?;
    }

    let mut modified = Vec::new();
    let mut new_files = Vec::new();

    for path in files {
        let data = fs::read(path).with_context(|| format!("failed to read {path}"))?;
        let blob = Blob { data };
        let wt_hash = object::hash_bytes(&object::encode(&blob)?);
        match head_files.remove(path) {
            Some(head_hash) if head_hash == wt_hash => {}
            Some(_) => modified.push(path.clone()),
            None => new_files.push(path.clone()),
        }
    }
    let mut deleted: Vec<String> = head_files.into_keys().collect();
    deleted.sort();

    Ok(WorkingChanges {
        modified,
        new_files,
        deleted,
    })
}

fn diff() -> Result<()> {
    let repo_db = local_db_path_existing()?;
    let conn = open_repo(&repo_db)?;
    let head = read_head(&conn)?;
    let files = scan_working_tree(Path::new("."))?;

    let mut head_files: std::collections::HashMap<String, ObjectHash> =
        std::collections::HashMap::new();
    if let Some(head_hash) = &head {
        let head_commit = object::load_commit(&conn, head_hash)?;
        collect_tree_files(&conn, &head_commit.tree, "", &mut head_files)?;
    }

    for path in &files {
        let wt_bytes = fs::read(path).with_context(|| format!("failed to read {path}"))?;
        let blob = Blob { data: wt_bytes.clone() };
        let wt_hash = object::hash_bytes(&object::encode(&blob)?);
        match head_files.remove(path) {
            Some(head_hash) if head_hash == wt_hash => {}
            Some(head_hash) => {
                let head_blob = object::load_blob(&conn, &head_hash)?;
                emit_diff(path, Some(&head_blob.data), Some(&wt_bytes));
            }
            None => {
                emit_diff(path, None, Some(&wt_bytes));
            }
        }
    }

    let mut deleted: Vec<(String, ObjectHash)> = head_files.into_iter().collect();
    deleted.sort_by(|a, b| a.0.cmp(&b.0));
    for (path, head_hash) in deleted {
        let head_blob = object::load_blob(&conn, &head_hash)?;
        emit_diff(&path, Some(&head_blob.data), None);
    }

    Ok(())
}

fn emit_diff(path: &str, old: Option<&[u8]>, new: Option<&[u8]>) {
    println!("diff --cntl a/{path} b/{path}");

    let binary = old.is_some_and(is_binary) || new.is_some_and(is_binary);
    if binary {
        println!("Binary files a/{path} and b/{path} differ");
        return;
    }

    let old_text = old.and_then(|b| std::str::from_utf8(b).ok()).unwrap_or("");
    let new_text = new.and_then(|b| std::str::from_utf8(b).ok()).unwrap_or("");

    let old_label = if old.is_some() {
        format!("a/{path}")
    } else {
        "/dev/null".to_string()
    };
    let new_label = if new.is_some() {
        format!("b/{path}")
    } else {
        "/dev/null".to_string()
    };

    let diff = similar::TextDiff::from_lines(old_text, new_text);
    let unified = diff
        .unified_diff()
        .header(&old_label, &new_label)
        .to_string();
    print!("{unified}");
}

fn is_binary(bytes: &[u8]) -> bool {
    let check_len = bytes.len().min(8192);
    bytes[..check_len].contains(&0)
}

fn restore(paths: Vec<String>) -> Result<()> {
    let repo_db = local_db_path_existing()?;
    let conn = open_repo(&repo_db)?;
    let head_hash = match read_head(&conn)? {
        Some(h) => h,
        None => bail!("no commits yet"),
    };
    let head_commit = object::load_commit(&conn, &head_hash)?;

    let mut head_files: std::collections::HashMap<String, ObjectHash> =
        std::collections::HashMap::new();
    collect_tree_files(&conn, &head_commit.tree, "", &mut head_files)?;

    // Phase 1: validate all paths before touching the working tree.
    let mut resolved: Vec<(String, ObjectHash)> = Vec::new();
    for raw in &paths {
        let path = normalize_restore_path(raw);
        if let Some(hash) = head_files.get(&path) {
            resolved.push((path, *hash));
        } else if is_dir_in_head(&head_files, &path) {
            bail!("{path}: is a directory (not yet supported)");
        } else {
            bail!("{path}: not in HEAD");
        }
    }

    // Phase 2: write each file from its HEAD blob.
    for (path, hash) in &resolved {
        let blob = object::load_blob(&conn, hash)?;
        let p = Path::new(path);
        if let Some(parent) = p.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create parent of {path}"))?;
            }
        }
        fs::write(p, &blob.data).with_context(|| format!("failed to write {path}"))?;
    }

    Ok(())
}

fn normalize_restore_path(p: &str) -> String {
    let p = p.strip_prefix("./").unwrap_or(p);
    p.trim_end_matches('/').to_string()
}

fn is_dir_in_head(files: &std::collections::HashMap<String, ObjectHash>, dir: &str) -> bool {
    let prefix = format!("{dir}/");
    files.keys().any(|k| k.starts_with(&prefix))
}

/// Read the branch ref that HEAD points at.
///
/// HEAD is a symbolic reference (see DR-002). The post-v0.2.0 invariant is
/// that exactly one row exists in the `head` table.
fn read_head_ref(conn: &Connection) -> Result<String> {
    conn.query_row("SELECT ref_name FROM head WHERE id = 1", [], |row| row.get(0))
        .context("failed to read HEAD")
}

/// Resolve HEAD to the commit it points at, or `None` if the branch ref
/// does not exist yet (i.e. no commits have been made).
fn read_head(conn: &Connection) -> Result<Option<ObjectHash>> {
    let head_ref = read_head_ref(conn)?;
    read_ref(conn, &head_ref)
}

/// Read an arbitrary ref by name; returns `None` if the ref does not exist.
fn read_ref(conn: &Connection, name: &str) -> Result<Option<ObjectHash>> {
    conn.query_row(
        "SELECT target FROM refs WHERE name = ?1",
        params![name],
        |row| {
            let bytes: Vec<u8> = row.get(0)?;
            bytes.try_into().map_err(|_| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Blob,
                    format!("ref {name} target is not a 32-byte hash").into(),
                )
            })
        },
    )
    .optional()
    .with_context(|| format!("failed to read ref {name}"))
}

fn collect_tree_files(
    conn: &Connection,
    tree_hash: &ObjectHash,
    prefix: &str,
    out: &mut std::collections::HashMap<String, ObjectHash>,
) -> Result<()> {
    let tree = object::load_tree(conn, tree_hash)?;
    for entry in tree.entries {
        let path = if prefix.is_empty() {
            entry.name
        } else {
            format!("{prefix}/{}", entry.name)
        };
        match entry.kind {
            TreeEntryKind::Blob => {
                out.insert(path, entry.hash);
            }
            TreeEntryKind::Tree => {
                collect_tree_files(conn, &entry.hash, &path, out)?;
            }
        }
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

    let mut conn = open_repo(&repo_db)?;
    let tx = conn.transaction().context("failed to begin transaction")?;

    let tree_hash = write_tree_recursive(&tx, Path::new("."))?;
    let head_ref = read_head_ref(&tx)?;
    let parent = read_ref(&tx, &head_ref)?;

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
        "INSERT INTO refs (name, target) VALUES (?1, ?2)
         ON CONFLICT(name) DO UPDATE SET target = excluded.target",
        params![head_ref, &commit_hash[..]],
    )
    .context("failed to advance branch ref")?;

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

fn branch(name: Option<String>, delete: bool) -> Result<()> {
    let repo_db = local_db_path_existing()?;
    let conn = open_repo(&repo_db)?;

    match (name, delete) {
        (None, _) => branch_list(&conn),
        (Some(n), false) => branch_create(&conn, &n),
        (Some(n), true) => branch_delete(&conn, &n),
    }
}

fn branch_list(conn: &Connection) -> Result<()> {
    let current = read_head_ref(conn)?;
    let mut stmt = conn
        .prepare("SELECT name FROM refs WHERE name LIKE 'refs/heads/%' ORDER BY name")
        .context("failed to prepare branch list query")?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .context("failed to query branches")?;

    let mut any = false;
    for row in rows {
        let full = row.context("failed to read branch row")?;
        let short = full.strip_prefix(BRANCH_REF_PREFIX).unwrap_or(&full);
        let marker = if full == current { "*" } else { " " };
        println!("{marker} {short}");
        any = true;
    }

    if !any {
        let current_short = current.strip_prefix(BRANCH_REF_PREFIX).unwrap_or(&current);
        println!("No branches yet (HEAD will become '{current_short}' at first commit).");
    }
    Ok(())
}

fn branch_create(conn: &Connection, name: &str) -> Result<()> {
    validate_branch_name(name)?;
    let full = format!("{BRANCH_REF_PREFIX}{name}");

    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM refs WHERE name = ?1)",
            params![full],
            |row| row.get(0),
        )
        .context("failed to check existing branch")?;
    if exists {
        bail!("branch '{name}' already exists");
    }

    let head_hash = read_head(conn)?
        .ok_or_else(|| anyhow::anyhow!("no commits yet — cannot create a branch"))?;

    conn.execute(
        "INSERT INTO refs (name, target) VALUES (?1, ?2)",
        params![full, &head_hash[..]],
    )
    .context("failed to create branch ref")?;
    Ok(())
}

fn branch_delete(conn: &Connection, name: &str) -> Result<()> {
    validate_branch_name(name)?;
    let full = format!("{BRANCH_REF_PREFIX}{name}");

    let current = read_head_ref(conn)?;
    if full == current {
        bail!("cannot delete branch '{name}': it is the current branch");
    }

    let deleted = conn
        .execute("DELETE FROM refs WHERE name = ?1", params![full])
        .context("failed to delete branch ref")?;
    if deleted == 0 {
        bail!("branch '{name}' not found");
    }
    Ok(())
}

fn validate_branch_name(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("branch name cannot be empty");
    }
    if name.starts_with('-') {
        bail!("branch name cannot start with '-'");
    }
    if name == "." || name == ".." {
        bail!("branch name cannot be '.' or '..'");
    }
    if name.chars().any(|c| c.is_control() || c == '/' || c == ' ' || c == '\t') {
        bail!("branch name contains invalid characters (no '/', spaces, or control chars)");
    }
    Ok(())
}

fn checkout(name: &str) -> Result<()> {
    validate_branch_name(name)?;
    let repo_db = local_db_path_existing()?;
    let conn = open_repo(&repo_db)?;

    let target_ref = format!("{BRANCH_REF_PREFIX}{name}");
    let Some(target_commit) = read_ref(&conn, &target_ref)? else {
        bail!("branch '{name}' not found");
    };

    let current_ref = read_head_ref(&conn)?;
    if current_ref == target_ref {
        println!("Already on '{name}'");
        return Ok(());
    }

    // DR-002: switching branches must never silently discard work. Refuse the
    // checkout unless the working tree is clean relative to the current branch.
    let current_commit = read_ref(&conn, &current_ref)?;
    let files = scan_working_tree(Path::new("."))?;
    let changes = compute_working_changes(&conn, current_commit.as_ref(), &files)?;
    if !changes.is_clean() {
        bail!("{}", dirty_checkout_message(&changes));
    }

    // The working tree is now known to match the current branch's commit
    // exactly, so the file set on disk is precisely `current_files`. Move it to
    // `target_files`, touching only the paths whose content actually differs.
    let mut current_files: std::collections::HashMap<String, ObjectHash> =
        std::collections::HashMap::new();
    if let Some(commit_hash) = &current_commit {
        let commit = object::load_commit(&conn, commit_hash)?;
        collect_tree_files(&conn, &commit.tree, "", &mut current_files)?;
    }
    let mut target_files: std::collections::HashMap<String, ObjectHash> =
        std::collections::HashMap::new();
    let target = object::load_commit(&conn, &target_commit)?;
    collect_tree_files(&conn, &target.tree, "", &mut target_files)?;

    for (path, hash) in &target_files {
        if current_files.get(path) == Some(hash) {
            continue;
        }
        let blob = object::load_blob(&conn, hash)?;
        let p = Path::new(path);
        if let Some(parent) = p.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create parent of {path}"))?;
            }
        }
        fs::write(p, &blob.data).with_context(|| format!("failed to write {path}"))?;
    }

    for path in current_files.keys() {
        if target_files.contains_key(path) {
            continue;
        }
        let p = Path::new(path);
        fs::remove_file(p).with_context(|| format!("failed to remove {path}"))?;
        remove_empty_parents(p);
    }

    conn.execute(
        "UPDATE head SET ref_name = ?1 WHERE id = 1",
        params![target_ref],
    )
    .context("failed to update HEAD")?;

    println!("Switched to branch '{name}'");
    Ok(())
}

/// Build the multi-line error shown when a checkout is refused because the
/// working tree has uncommitted changes.
fn dirty_checkout_message(changes: &WorkingChanges) -> String {
    use std::fmt::Write;
    let mut msg = String::from("working tree has uncommitted changes");
    for path in &changes.modified {
        let _ = write!(msg, "\n  modified: {path}");
    }
    for path in &changes.new_files {
        let _ = write!(msg, "\n  new file: {path}");
    }
    for path in &changes.deleted {
        let _ = write!(msg, "\n  deleted:  {path}");
    }
    msg.push_str("\nhint: commit them first, then checkout");
    msg
}

/// Remove now-empty parent directories of a just-deleted file, walking upward
/// until a non-empty directory (or the working-tree root) is reached.
fn remove_empty_parents(path: &Path) {
    let mut dir = path.parent();
    while let Some(d) = dir {
        if d.as_os_str().is_empty() {
            break;
        }
        // `remove_dir` only succeeds on an empty directory; any error (not
        // empty, already gone, permission) is the natural stopping point.
        if fs::remove_dir(d).is_err() {
            break;
        }
        dir = d.parent();
    }
}

fn log() -> Result<()> {
    let repo_db = local_db_path_existing()?;
    let conn = open_repo(&repo_db)?;

    let Some(head_hash) = read_head(&conn)? else {
        bail!("no commits yet");
    };

    let mut current = Some(head_hash);
    let mut first = true;
    while let Some(hash) = current {
        let commit = object::load_commit(&conn, &hash)?;
        if !first {
            println!();
        }
        print_commit(&hash, &commit);
        first = false;
        current = commit.parent;
    }
    Ok(())
}

fn print_commit(hash: &ObjectHash, commit: &Commit) {
    let utc = chrono::DateTime::<chrono::Utc>::from_timestamp(commit.timestamp_utc, 0)
        .unwrap_or_else(chrono::Utc::now);
    let local = utc.with_timezone(&chrono::Local);
    println!("commit {}", hex_full(hash));
    println!(
        "Author: {} <{}>",
        commit.author_name, commit.author_email
    );
    println!("Date:   {}", local.format("%Y-%m-%d %H:%M:%S %z"));
    println!();
    for line in commit.message.lines() {
        println!("    {line}");
    }
}

fn hex_full(hash: &ObjectHash) -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(64);
    for b in hash {
        let _ = write!(s, "{b:02x}");
    }
    s
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
