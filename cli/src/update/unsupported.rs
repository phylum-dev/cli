//! Dummy functions for platforms where self-update is unsupported

/// Check if a newer version of the client is available
pub async fn needs_update(current_version: &str, prerelease: bool) -> bool {
    false
}

/// Perform a self-update to the latest version
pub async fn do_update(prerelease: bool) -> anyhow::Result<String> {
    anyhow::bail!("Self-update not supported on this platform")
}
