//! Dummy functions for platforms where self-update is unsupported

/// Check if a newer version of the client is available
pub async fn needs_update(_prerelease: bool) -> bool {
    false
}

/// Perform a self-update to the latest version
pub async fn do_update(_prerelease: bool, _ignore_certs: bool) -> anyhow::Result<String> {
    anyhow::bail!("Self-update not supported on this platform")
}
