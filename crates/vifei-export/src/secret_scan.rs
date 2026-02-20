use crate::scanner::{redact_match, scan_bytes, scan_text, SecretPatterns};
use crate::{BlockedItem, DiscoveredContent};
use std::io;
use vifei_core::blob_store::BlobStore;
use vifei_core::event::CommittedEvent;

/// Scan discovered content for secrets.
///
/// Scans all event payloads and blob contents for secret patterns.
/// Returns a list of blocked items. Empty list means clean.
pub(crate) fn scan_for_secrets(
    content: &DiscoveredContent,
    blob_store: Option<&BlobStore>,
) -> io::Result<Vec<BlockedItem>> {
    let patterns = SecretPatterns::new();
    let mut items = Vec::new();

    // Scan event payloads
    for event in &content.events {
        let event_items = scan_event(&patterns, event);
        items.extend(event_items);
    }

    // Scan blob contents
    if let Some(store) = blob_store {
        for blob_ref in &content.blob_refs {
            if let Some(blob_data) = store.read_blob(blob_ref)? {
                let blob_items = scan_blob(&patterns, blob_ref, &blob_data);
                items.extend(blob_items);
            }
        }
    }

    Ok(items)
}

/// Scan a single event for secrets.
fn scan_event(patterns: &SecretPatterns, event: &CommittedEvent) -> Vec<BlockedItem> {
    let mut items = Vec::new();

    // Serialize the payload to JSON for scanning
    let payload_json = match serde_json::to_string(&event.payload) {
        Ok(json) => json,
        Err(_) => return items,
    };

    // Scan the payload JSON
    for m in scan_text(patterns, &payload_json) {
        items.push(BlockedItem {
            event_id: event.event_id.clone(),
            field_path: "payload".into(),
            matched_pattern: m.pattern_name,
            blob_ref: None,
            redacted_match: redact_match(&m.matched_text),
        });
    }

    items
}

/// Scan a blob for secrets.
fn scan_blob(patterns: &SecretPatterns, blob_ref: &str, data: &[u8]) -> Vec<BlockedItem> {
    let mut items = Vec::new();

    for m in scan_bytes(patterns, data) {
        items.push(BlockedItem {
            event_id: String::new(),
            field_path: "content".into(),
            matched_pattern: m.pattern_name,
            blob_ref: Some(blob_ref.to_string()),
            redacted_match: redact_match(&m.matched_text),
        });
    }

    items
}
