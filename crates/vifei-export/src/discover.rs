use crate::DiscoveredContent;
use std::collections::HashSet;
use std::io;
use std::path::Path;
use vifei_core::eventlog::read_eventlog;

/// Discover all content referenced by an EventLog.
///
/// Reads the EventLog and identifies all blob references.
pub(crate) fn discover_content(eventlog_path: &Path) -> io::Result<DiscoveredContent> {
    let events = read_eventlog(eventlog_path)?;
    let mut blob_refs = HashSet::new();

    for event in &events {
        if let Some(ref payload_ref) = event.payload_ref {
            blob_refs.insert(payload_ref.clone());
        }
    }

    Ok(DiscoveredContent {
        eventlog_path: eventlog_path.to_path_buf(),
        events,
        blob_refs,
    })
}
