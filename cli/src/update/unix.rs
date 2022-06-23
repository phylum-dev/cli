use std::io::{self, Cursor, Write};
use std::process::Command;
use std::str;

use anyhow::{anyhow, Context};
use log::debug;
use minisign_verify::{PublicKey, Signature};
use reqwest::Client;
use serde::de::DeserializeOwned;
use spinners::{Spinner, Spinners};
#[cfg(test)]
use wiremock::MockServer;
use zip::ZipArchive;

use crate::types::{GithubRelease, GithubReleaseAsset};

// Phylum's public key for Minisign.
const PUBKEY: &str = "RWT6G44ykbS8GABiLXrJrYsap7FCY77m/Jyi0fgsr/Fsy3oLwU4l0IDf";

const GITHUB_URI: &str = "https://api.github.com";

// For updates, we use the cargo version instead of git_version
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Check if a newer version of the client is available
pub async fn needs_update(prerelease: bool) -> bool {
    let updater = ApplicationUpdater::default();
    match updater.get_latest_version(prerelease).await {
        Ok(latest) => updater.needs_update(CURRENT_VERSION, &latest),
        Err(e) => {
            log::debug!("Failed to get the latest version for update check: {:?}", e);
            false
        },
    }
}

/// Perform a self-update to the latest version
pub async fn do_update(prerelease: bool) -> anyhow::Result<String> {
    let updater = ApplicationUpdater::default();
    let ver =
        updater.get_latest_version(prerelease).await.context("Failed to get the latest version")?;
    updater.do_update(ver).await.map(|ver| format!("Successfully updated to {}!", ver.name))
}

#[derive(Debug)]
struct ApplicationUpdater {
    pubkey: PublicKey,
    github_uri: String,
}

impl Default for ApplicationUpdater {
    fn default() -> Self {
        let pubkey = PublicKey::from_base64(PUBKEY).expect("Unable to decode the public key");
        ApplicationUpdater { pubkey, github_uri: GITHUB_URI.to_owned() }
    }
}

/// Generic function for fetching data via HTTP GET.
async fn http_get(url: &str) -> anyhow::Result<reqwest::Response> {
    let client = Client::builder().user_agent("phylum-cli").build()?;
    let response = client.get(url).send().await?;
    Ok(response)
}

/// Generic function for fetching JSON structs via HTTP GET.
async fn http_get_json<T: DeserializeOwned>(url: &str) -> anyhow::Result<T> {
    let response = http_get(url).await?;

    if let Err(error) = response.error_for_status_ref() {
        Err(anyhow!(response.text().await?)).context(error)
    } else {
        Ok(response.json().await?)
    }
}

/// Download the specified Github asset. Returns a bytes object containing the
/// contents of the asset.
async fn download_github_asset(latest: &GithubReleaseAsset) -> anyhow::Result<bytes::Bytes> {
    let r = http_get(&latest.browser_download_url).await?;
    Ok(r.bytes().await?)
}

const SUPPORTED_PLATFORMS: &[&str] =
    &["x86_64-unknown-linux-musl", "x86_64-apple-darwin", "aarch64-apple-darwin"];

/// Determine the current platform. Error if unsupported.
fn current_platform() -> anyhow::Result<String> {
    let arch = if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        "unsupported"
    };
    let os = if cfg!(target_os = "linux") {
        // We could check cfg!(target_env = "musl") here, but I think that's
        // unnecessary. If a user compiles the CLI for x86_64-unknown-linux-gnu
        // and then runs `phylum update`, we should be able to upgrade them to a
        // x86_64-unknown-linux-musl binary without breaking anything.
        "unknown-linux-musl"
    } else if cfg!(target_os = "macos") {
        "apple-darwin"
    } else {
        "unsupported"
    };

    let platform = format!("{}-{}", arch, os);
    if SUPPORTED_PLATFORMS.contains(&platform.as_str()) {
        Ok(platform)
    } else {
        Err(anyhow::anyhow!("unsupported platform: {}", platform))
    }
}

/// Utility for handling updating the Phylum installation in place, along with
/// facilities for validating the binary signature before installation.
impl ApplicationUpdater {
    /// Build a instance for use in tests
    #[cfg(test)]
    fn build_test_instance(mock_server: MockServer) -> Self {
        let pubkey = PublicKey::from_base64(PUBKEY).expect("Unable to decode the public key");
        ApplicationUpdater { pubkey, github_uri: mock_server.uri() }
    }

    /// Check for an update by querying the Github releases page.
    async fn get_latest_version(&self, prerelease: bool) -> anyhow::Result<GithubRelease> {
        let ver = if prerelease {
            let url = format!("{}/repos/phylum-dev/cli/releases", self.github_uri);
            let releases = http_get_json::<Vec<GithubRelease>>(&url).await?;
            // Use the first one in the list, which should be the most recent
            releases.first().cloned().ok_or_else(|| anyhow::anyhow!("no releases found"))?
        } else {
            let url = format!("{}/repos/phylum-dev/cli/releases/latest", self.github_uri);
            http_get_json::<GithubRelease>(&url).await?
        };

        log::debug!("Found latest version: {:?}", ver);

        Ok(ver)
    }

    /// Compare the current version as reported by Clap with the version
    /// currently published on Github. We do the naive thing here: If the
    /// latest version on Github does not match the Clap version, we
    /// indicate that we need to update. We do not compare semvers to
    /// determine if an update is required.
    fn needs_update(&self, current_version: &str, latest_version: &GithubRelease) -> bool {
        let latest = latest_version.name.replace("phylum ", "").replace('v', "").trim().to_owned();
        let current = current_version.replace("phylum ", "").trim().to_owned();
        latest != current
    }

    /// Locate the specified asset in the Github response structure.
    fn find_github_asset<'a>(
        &self,
        latest: &'a GithubRelease,
        name: &str,
    ) -> Result<&'a GithubReleaseAsset, io::Error> {
        match latest.assets.iter().find(|x| x.name == name) {
            Some(x) => Ok(x),
            _ => Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to download update file: {}", name),
            )),
        }
    }

    /// Update the Phylum installation. Please note, this will only function on
    /// Linux and macOS x64. This is due in part to the fact that the release is
    /// only compiling for these OSes and architectures.
    ///
    /// Until we update the releases, this should suffice.
    async fn do_update(&self, latest: GithubRelease) -> anyhow::Result<GithubRelease> {
        let mut spinner = Spinner::new(
            Spinners::Dots12,
            "Downloading update and verifying binary signatures...".into(),
        );
        debug!("Performing the update process");

        let archive_name = format!("phylum-{}", current_platform()?);

        // Get the URL for each asset from the Github JSON response in `latest`.
        debug!("Finding the github assets in the Github JSON response");
        let zip_asset = self.find_github_asset(&latest, &format!("{}.zip", archive_name))?;
        let sig_asset =
            self.find_github_asset(&latest, &format!("{}.zip.minisig", archive_name))?;

        debug!("Downloading the update files");
        let zip = download_github_asset(zip_asset).await?;
        let sig = download_github_asset(sig_asset).await?;

        debug!("Verifying the package signature");
        if !self.has_valid_signature(&zip, str::from_utf8(&sig)?) {
            anyhow::bail!("The update binary failed signature validation");
        }
        spinner.stop_with_message(
            "Downloading update and verifying binary signatures... Done!".into(),
        );
        std::io::stdout().flush()?;

        debug!("Extracting package to temporary directory");
        let temp_dir = tempfile::tempdir()?;
        ZipArchive::new(Cursor::new(zip))?.extract(temp_dir.path())?;

        debug!("Running the installer");
        let working_dir = temp_dir.path().join(archive_name);
        let status =
            Command::new(working_dir.join("install.sh")).current_dir(&working_dir).status()?;
        anyhow::ensure!(status.success(), "install.sh returned failure");

        Ok(latest)
    }

    /// Verify that the downloaded binary matches the expected signature.
    /// Returns `true` for a valid signature, `false` otherwise.
    fn has_valid_signature(&self, bin: &[u8], sig: &str) -> bool {
        let signature = match Signature::decode(sig) {
            Ok(x) => x,
            Err(_) => return false,
        };

        // TODO: Change the `allow_legacy` argument to `false` after we stop using
        // legacy signatures https://github.com/phylum-dev/cli/issues/266
        self.pubkey.verify(bin, &signature, true).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use minisign_verify::PublicKey;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, ResponseTemplate};

    use super::ApplicationUpdater;
    use crate::test::mockito::*;

    #[test]
    fn creating_application() {
        let correct_pubkey =
            PublicKey::from_base64("RWT6G44ykbS8GABiLXrJrYsap7FCY77m/Jyi0fgsr/Fsy3oLwU4l0IDf")
                .expect("Failed to create public key");
        let updater = ApplicationUpdater::default();
        assert!(correct_pubkey == updater.pubkey);
    }

    #[tokio::test]
    async fn version_check() {
        let body = r#"{
            "name": "1.2.3",
            "assets": [
              { "browser_download_url": "https://foo.example.com", "name": "foo" },
              { "browser_download_url": "https://bar.example.com", "name": "bar" }
            ]
          }"#;

        let mock_server = build_mock_server().await;
        Mock::given(method("GET"))
            .and(path("/repos/phylum-dev/cli/releases/latest"))
            .respond_with_fn(move |_| ResponseTemplate::new(200).set_body_string(body))
            .mount(&mock_server)
            .await;

        let updater = ApplicationUpdater::build_test_instance(mock_server);
        let latest = updater.get_latest_version(false).await.unwrap();
        log::error!("{:?}", latest);
        assert!("1.2.3" == latest.name);
        assert!(updater.needs_update("1.0.2", &latest));

        let github_asset = updater.find_github_asset(&latest, "foo").unwrap();
        assert!("https://foo.example.com" == github_asset.browser_download_url);
    }

    #[test]
    fn test_signature_validation() {
        let file = b"Hello, world\n";
        let minisign_sig =
            "untrusted comment: signature from minisign secret \
             key\nRWT6G44ykbS8GJ+2A+Fjj6ZdR1/\
             632p6WlwqAYhb8DSeKhCl3rzG1TGSF9CD9DDf9BdWrOjvnqi78yh38djVuYvAW2FhE0MvTQ4=\ntrusted \
             comment: Phylum, Inc. - Future of software supply chain \
             security\nkBL1siaOp2uZq2IrNKVguDGje88ghM2L0XJ6n/\
             1rjGL2aQwbJ0fZPe5uOde3IbObPKTF4KCHbRtMALUEu6TaBQ==\n";

        let updater = ApplicationUpdater::default();
        assert!(updater.has_valid_signature(file, minisign_sig));
    }
}
