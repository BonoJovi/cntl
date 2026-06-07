use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

/// blake3 digest (32 bytes). Identifies every stored object.
pub type ObjectHash = [u8; 32];

/// Snapshot of a single file's bytes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blob {
    pub data: Vec<u8>,
}

/// What a tree entry points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TreeEntryKind {
    Blob,
    Tree,
}

/// One name -> object mapping inside a tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeEntry {
    pub name: String,
    pub kind: TreeEntryKind,
    pub hash: ObjectHash,
}

/// Snapshot of a directory: a sorted list of entries.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Tree {
    pub entries: Vec<TreeEntry>,
}

/// Commit: tree snapshot + author + parent + message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    pub parent: Option<ObjectHash>,
    pub tree: ObjectHash,
    pub author_name: String,
    pub author_email: String,
    /// Seconds since the Unix epoch, UTC.
    pub timestamp_utc: i64,
    pub message: String,
}

/// blake3 hash of a byte slice, returned as a 32-byte array.
pub fn hash_bytes(data: &[u8]) -> ObjectHash {
    *blake3::hash(data).as_bytes()
}

/// Serialize a value with bincode using the standard configuration.
pub fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    bincode::serde::encode_to_vec(value, bincode::config::standard())
        .context("failed to encode object")
}

/// Insert an object into the `objects` table; no-op if the hash is already present.
pub fn store_object(
    conn: &Connection,
    hash: &ObjectHash,
    obj_type: &str,
    data: &[u8],
) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO objects (hash, obj_type, data) VALUES (?1, ?2, ?3)",
        params![&hash[..], obj_type, data],
    )
    .context("failed to store object")?;
    Ok(())
}
