use crate::{BundleManifest, DiscoveredContent, ExportSuccess, ManifestEntry};
use std::io;
use std::path::Path;
use vifei_core::blob_store::BlobStore;
use vifei_core::projection::PROJECTION_INVARIANTS_VERSION;

/// Bundle discovered content into a deterministic tar+zstd archive.
///
/// Determinism requirements (CAPACITY_ENVELOPE Export determinism targets):
/// - Tar format: POSIX (UStar/PAX-compatible)
/// - Zstd compression level: 3 (pinned, not library default)
/// - Tar mtime: 0 (Unix epoch, all entries normalized)
/// - Tar uid/gid: 0 (normalized to prevent machine-specific values)
/// - Tar username/groupname: empty
/// - Entries sorted alphabetically by path
/// - bundle_hash = BLAKE3 of final .tar.zst bytes
pub(crate) fn create_bundle(
    content: &DiscoveredContent,
    blob_store: Option<&BlobStore>,
    output_path: &Path,
) -> io::Result<ExportSuccess> {
    // Collect all entries as (archive_path, data) for deterministic sorting
    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();

    // Add EventLog
    let eventlog_bytes = std::fs::read(&content.eventlog_path)?;
    entries.push(("eventlog.jsonl".to_string(), eventlog_bytes));

    // Add blobs (sorted by ref for deterministic ordering)
    let mut blob_count = 0usize;
    if let Some(store) = blob_store {
        let mut sorted_refs: Vec<&str> = content.blob_refs.iter().map(|s| s.as_str()).collect();
        sorted_refs.sort();
        for blob_ref in sorted_refs {
            if let Some(data) = store.read_blob(blob_ref)? {
                entries.push((format!("blobs/{}", blob_ref), data));
                blob_count += 1;
            }
        }
    }

    // Sort all entries alphabetically by path (deterministic archive order)
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    // Build manifest entries with BLAKE3 digests for each file
    let manifest_file_entries: Vec<ManifestEntry> = entries
        .iter()
        .map(|(path, data)| ManifestEntry {
            path: path.clone(),
            blake3: blake3::hash(data).to_hex().to_string(),
            size: data.len() as u64,
        })
        .collect();

    // Compute commit_index range from events
    let commit_index_range = content.events.iter().map(|event| event.commit_index).fold(
        None,
        |acc: Option<[u64; 2]>, idx| match acc {
            Some([min_idx, max_idx]) => Some([min_idx.min(idx), max_idx.max(idx)]),
            None => Some([idx, idx]),
        },
    );

    // Build the manifest
    let manifest = BundleManifest {
        manifest_version: "manifest-v0.1".to_string(),
        files: manifest_file_entries,
        commit_index_range,
        projection_invariants_version: PROJECTION_INVARIANTS_VERSION.to_string(),
    };
    let manifest_json = serde_json::to_string_pretty(&manifest).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("manifest serialization: {e}"),
        )
    })?;

    // Add manifest to entries (will be sorted into correct position)
    entries.push(("manifest.json".to_string(), manifest_json.into_bytes()));
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    // Build tar+zstd into a memory buffer so we can BLAKE3-hash the result
    let mut compressed_bytes: Vec<u8> = Vec::new();
    {
        // Zstd level 3 (pinned per CAPACITY_ENVELOPE)
        let encoder = zstd::stream::write::Encoder::new(&mut compressed_bytes, 3)
            .map_err(|e| io::Error::other(format!("zstd init: {e}")))?;
        let mut tar_builder = tar::Builder::new(encoder);

        for (path, data) in &entries {
            append_tar_entry(&mut tar_builder, path, data)?;
        }

        // Finish tar (writes final blocks), then finish zstd (flushes frame)
        let encoder = tar_builder.into_inner()?;
        encoder.finish()?;
    }

    // bundle_hash = BLAKE3 of final .tar.zst bytes
    let bundle_hash = blake3::hash(&compressed_bytes).to_hex().to_string();

    // Write the completed bundle to disk
    std::fs::write(output_path, &compressed_bytes)?;

    Ok(ExportSuccess {
        bundle_path: output_path.to_path_buf(),
        bundle_hash,
        event_count: content.event_count(),
        blob_count,
    })
}

/// Append a single entry to a tar archive with normalized metadata.
///
/// All metadata is normalized per CAPACITY_ENVELOPE Export determinism targets.
fn append_tar_entry<W: io::Write>(
    builder: &mut tar::Builder<W>,
    path: &str,
    data: &[u8],
) -> io::Result<()> {
    let mut header = tar::Header::new_ustar();
    header.set_size(data.len() as u64);
    header.set_mtime(0);
    header.set_uid(0);
    header.set_gid(0);
    header.set_mode(0o644);
    header
        .set_username("")
        .map_err(|e| io::Error::other(format!("set_username: {e}")))?;
    header
        .set_groupname("")
        .map_err(|e| io::Error::other(format!("set_groupname: {e}")))?;
    header.set_entry_type(tar::EntryType::Regular);
    header.set_cksum();
    builder.append_data(&mut header, path, data)?;
    Ok(())
}
