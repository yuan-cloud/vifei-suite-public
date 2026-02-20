//! Content-addressed blob store for payloads exceeding the inline threshold.
//!
//! # Overview
//!
//! When an event payload exceeds the inline payload max bytes threshold
//! (see `docs/CAPACITY_ENVELOPE.md`), the bytes are written to a blob file
//! and referenced by `payload_ref` — the lowercase hex BLAKE3 digest of
//! the blob bytes as stored on disk.
//!
//! # Layout
//!
//! ```text
//! blobs/
//!   {first-2-hex-chars}/
//!     {full-64-char-blake3-hex}
//! ```
//!
//! The two-character prefix directory reduces per-directory inode pressure.
//!
//! # Deduplication
//!
//! Content-addressing provides natural deduplication: if the same payload
//! is stored twice, the second write is a no-op (the file already exists).
//!
//! # Error handling
//!
//! Blob write failures (fsync error, timeout beyond the blob fsync timeout
//! budget in `docs/CAPACITY_ENVELOPE.md`) correspond to failure mode
//! `FM-BLOB-WRITE-FAIL` in `docs/BACKPRESSURE_POLICY.md`. The caller
//! (append writer) is responsible for entering L5 safe failure posture.
//!
//! # Invariants
//!
//! - **I1 (Forensic truth):** Blob bytes are stored exactly as received.
//! - **I5 (Loud failure):** Errors are returned, never silently swallowed.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Inline payload max bytes. Payloads at or below this size are stored
/// inline in the JSONL event. Above this threshold, they go to the blob
/// store. Value is from `docs/CAPACITY_ENVELOPE.md`.
pub const INLINE_PAYLOAD_MAX_BYTES: usize = 16_384;

/// Content-addressed blob store backed by the filesystem.
#[derive(Debug)]
pub struct BlobStore {
    /// Root directory for blob storage.
    root: PathBuf,
}

impl BlobStore {
    /// Create a new blob store rooted at `root`.
    ///
    /// Creates the root directory if it does not exist.
    pub fn open(root: impl Into<PathBuf>) -> io::Result<Self> {
        let root = root.into();
        fs::create_dir_all(&root)?;
        Ok(BlobStore { root })
    }

    /// Write payload bytes to the blob store.
    ///
    /// Returns the `payload_ref` — lowercase hex BLAKE3 digest of the
    /// stored bytes. If a blob with this digest already exists, the write
    /// is a no-op (content-addressed deduplication).
    pub fn write_blob(&self, data: &[u8]) -> io::Result<String> {
        let hash = blake3::hash(data);
        let hex = hash.to_hex();
        let payload_ref = hex.as_str();

        let blob_path = self.blob_path(payload_ref);

        // Deduplication: if blob already exists, skip the write.
        if blob_path.exists() {
            return Ok(payload_ref.to_string());
        }

        // Ensure the prefix directory exists.
        if let Some(parent) = blob_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write to a temp file then rename for atomicity.
        let tmp_path = blob_path.with_extension("tmp");
        let mut file = fs::File::create(&tmp_path)?;
        file.write_all(data)?;
        file.sync_all()?;
        fs::rename(&tmp_path, &blob_path)?;

        Ok(payload_ref.to_string())
    }

    /// Read blob bytes by `payload_ref` (BLAKE3 hex digest).
    ///
    /// Returns `None` if the blob does not exist.
    pub fn read_blob(&self, payload_ref: &str) -> io::Result<Option<Vec<u8>>> {
        if !Self::is_valid_payload_ref(payload_ref) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid payload_ref: {payload_ref:?}"),
            ));
        }
        let blob_path = self.blob_path(payload_ref);
        if !blob_path.exists() {
            return Ok(None);
        }
        let data = fs::read(&blob_path)?;
        Ok(Some(data))
    }

    /// Check if a blob exists.
    pub fn has_blob(&self, payload_ref: &str) -> bool {
        if !Self::is_valid_payload_ref(payload_ref) {
            return false;
        }
        self.blob_path(payload_ref).exists()
    }

    /// Returns true if `data` exceeds the inline payload threshold and
    /// should be stored as a blob.
    pub fn should_blob(data: &[u8]) -> bool {
        data.len() > INLINE_PAYLOAD_MAX_BYTES
    }

    /// Compute the BLAKE3 hex digest for the given bytes without storing.
    pub fn compute_ref(data: &[u8]) -> String {
        blake3::hash(data).to_hex().to_string()
    }

    /// Validate payload_ref format: 64 lowercase hex characters.
    fn is_valid_payload_ref(payload_ref: &str) -> bool {
        payload_ref.len() == 64
            && payload_ref
                .chars()
                .all(|c| matches!(c, '0'..='9' | 'a'..='f'))
    }

    /// Filesystem path for a blob given its `payload_ref`.
    fn blob_path(&self, payload_ref: &str) -> PathBuf {
        let prefix = &payload_ref[..2.min(payload_ref.len())];
        self.root.join(prefix).join(payload_ref)
    }

    /// Root directory of the blob store.
    pub fn root(&self) -> &Path {
        &self.root
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_and_read_blob() {
        let dir = tempfile::tempdir().unwrap();
        let store = BlobStore::open(dir.path().join("blobs")).unwrap();

        let data = b"hello blob world";
        let payload_ref = store.write_blob(data).unwrap();

        // payload_ref is lowercase hex BLAKE3
        assert_eq!(payload_ref.len(), 64);
        assert!(payload_ref.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(payload_ref, payload_ref.to_lowercase());

        // Read back
        let read_data = store.read_blob(&payload_ref).unwrap().unwrap();
        assert_eq!(read_data, data);
    }

    #[test]
    fn deduplication() {
        let dir = tempfile::tempdir().unwrap();
        let store = BlobStore::open(dir.path().join("blobs")).unwrap();

        let data = b"duplicate payload";
        let ref1 = store.write_blob(data).unwrap();
        let ref2 = store.write_blob(data).unwrap();

        assert_eq!(ref1, ref2, "same payload should produce same ref");
        assert!(store.has_blob(&ref1));
    }

    #[test]
    fn payload_ref_matches_blake3() {
        let data = b"verify hash independently";
        let expected = blake3::hash(data).to_hex().to_string();
        assert_eq!(BlobStore::compute_ref(data), expected);

        let dir = tempfile::tempdir().unwrap();
        let store = BlobStore::open(dir.path().join("blobs")).unwrap();
        let actual = store.write_blob(data).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn should_blob_threshold() {
        let at_threshold = vec![0u8; INLINE_PAYLOAD_MAX_BYTES];
        let above_threshold = vec![0u8; INLINE_PAYLOAD_MAX_BYTES + 1];

        assert!(
            !BlobStore::should_blob(&at_threshold),
            "at threshold = inline"
        );
        assert!(
            BlobStore::should_blob(&above_threshold),
            "above threshold = blob"
        );
    }

    #[test]
    fn read_nonexistent_blob() {
        let dir = tempfile::tempdir().unwrap();
        let store = BlobStore::open(dir.path().join("blobs")).unwrap();
        let result = store
            .read_blob("0000000000000000000000000000000000000000000000000000000000000000")
            .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn large_blob() {
        let dir = tempfile::tempdir().unwrap();
        let store = BlobStore::open(dir.path().join("blobs")).unwrap();

        let data = vec![b'x'; INLINE_PAYLOAD_MAX_BYTES + 1];
        let payload_ref = store.write_blob(&data).unwrap();

        let read_back = store.read_blob(&payload_ref).unwrap().unwrap();
        assert_eq!(read_back, data);
    }

    #[test]
    fn blob_path_uses_prefix_directory() {
        let dir = tempfile::tempdir().unwrap();
        let store = BlobStore::open(dir.path().join("blobs")).unwrap();

        let data = b"prefix test";
        let payload_ref = store.write_blob(data).unwrap();
        let prefix = &payload_ref[..2];

        let blob_path = store.blob_path(&payload_ref);
        assert!(blob_path.to_str().unwrap().contains(prefix));
        assert!(blob_path.exists());
    }

    #[test]
    fn invalid_payload_ref_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let store = BlobStore::open(dir.path().join("blobs")).unwrap();

        let err = store.read_blob("../etc/passwd").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        assert!(!store.has_blob("../etc/passwd"));
    }

    #[test]
    fn uppercase_payload_ref_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let store = BlobStore::open(dir.path().join("blobs")).unwrap();

        let data = b"case-check";
        let payload_ref = store.write_blob(data).unwrap();
        let uppercase = payload_ref.to_uppercase();

        let err = store.read_blob(&uppercase).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        assert!(!store.has_blob(&uppercase));
    }
}
