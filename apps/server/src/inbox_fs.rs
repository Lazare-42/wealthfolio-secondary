//! Filesystem helpers shared by the inbox watchers (email orders, PDF
//! statements, NAV envelopes): poll an inbox directory, then move each file to
//! a processed/ or failed/ sibling.

use std::path::Path;

use tracing::warn;

/// True when the path has a `.json` extension (case-insensitive).
pub fn is_json(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
}

/// Move `from` to `to`, creating the destination directory if needed.
/// Failures are logged, not returned: the file stays put and is retried on
/// the watcher's next poll.
pub async fn move_file(from: &Path, to: &Path) {
    if let Some(parent) = to.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }
    if let Err(err) = tokio::fs::rename(from, to).await {
        warn!("Failed to move {:?} -> {:?}: {}", from, to, err);
    }
}
