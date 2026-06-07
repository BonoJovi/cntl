use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;

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

/// Deserialize a value with bincode using the standard configuration.
pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    let (value, _len) = bincode::serde::decode_from_slice(bytes, bincode::config::standard())
        .context("failed to decode object")?;
    Ok(value)
}

fn load_raw(conn: &Connection, hash: &ObjectHash, expected_type: &str) -> Result<Vec<u8>> {
    let row: Option<(String, Vec<u8>)> = conn
        .query_row(
            "SELECT obj_type, data FROM objects WHERE hash = ?1",
            params![&hash[..]],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .context("failed to query object")?;
    let (obj_type, data) = row.with_context(|| format!("object not found: {}", hex_full(hash)))?;
    if obj_type != expected_type {
        bail!(
            "object {} has type {obj_type}, expected {expected_type}",
            hex_full(hash)
        );
    }
    Ok(data)
}

pub fn load_tree(conn: &Connection, hash: &ObjectHash) -> Result<Tree> {
    decode(&load_raw(conn, hash, "tree")?)
}

pub fn load_commit(conn: &Connection, hash: &ObjectHash) -> Result<Commit> {
    decode(&load_raw(conn, hash, "commit")?)
}

fn hex_full(hash: &ObjectHash) -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(64);
    for b in hash {
        let _ = write!(s, "{b:02x}");
    }
    s
}
