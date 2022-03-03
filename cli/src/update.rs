use std::env;
use std::fs;
use std::io;
use std::io::prelude::*;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::path::PathBuf;

use futures::Future;
use minisign_verify::{PublicKey, Signature};
use reqwest::{Client, Response};

#[cfg(test)]
use wiremock::MockServer;

use crate::types::{GithubRelease, GithubReleaseAsset};

// Phylum's public key for Minisign.
const PUBKEY: &str = "RWT6G44ykbS8GABiLXrJrYsap7FCY77m/Jyi0fgsr/Fsy3oLwU4l0IDf";

const GITHUB_URI: &str = "https://api.github.com";

#[derive(Debug)]
pub struct ApplicationUpdater {
    pubkey: PublicKey,
    github_uri: String,
}

impl Default for ApplicationUpdater {
    fn default() -> Self {
        let pubkey = PublicKey::from_base64(PUBKEY).expect("Unable to decode the public key");
        ApplicationUpdater {
            pubkey,
            github_uri: GITHUB_URI.to_owned(),
        }
    }
}

/// Produces the path to a temporary file on disk.
fn tmp_path(filename: &str) -> Option<String> {
    let tmp_loc = env::temp_dir();
    let path = Path::new(&tmp_loc);
    let tmp_path = path.join(filename);
    match tmp_path.into_os_string().into_string() {
        Ok(x) => Some(x),
        Err(_) => None,
    }
}

/// Utility for handling updating the Phylum installation in place, along with
/// facilities for validating the binary signature before installation.
impl ApplicationUpdater {
    /// Build a instance for use in tests
    #[cfg(test)]
    fn build_test_instance(mock_server: MockServer) -> Self {
        let pubkey = PublicKey::from_base64(PUBKEY).expect("Unable to decode the public key");
        ApplicationUpdater {
            pubkey,
            github_uri: mock_server.uri(),
        }
    }

    /// Locate the currently installed asset on the given host.
    fn installed_asset(
        &self,
        prefix: Option<&str>,
        asset_name: &str,
    ) -> Result<PathBuf, std::io::Error> {
        let mut current_bin = std::env::current_exe()?;
        current_bin.pop();
        if let Some(p) = prefix {
            current_bin.push(p);
        }
        current_bin.push(asset_name);
        Ok(current_bin)
    }

    /// Generic function for fetching data from Github.
    async fn get_github<T, C, F>(&self, url: &str, f: C) -> Option<T>
    where
        F: Future<Output = Option<T>>,
        C: Fn(Response) -> F,
    {
        // let q = move |r: i32| async move { r };

        let client = Client::builder().user_agent("phylum-cli").build();

        match client {
            Ok(client) => {
                let response = client.get(url).send().await;

                match response {
                    Ok(response) => f(response).await,
                    Err(_) => None,
                }
            }
            Err(_) => None,
        }
    }

    /// Check for an update by querying the Github releases page.
    pub async fn get_latest_version(&self, prerelease: bool) -> Option<GithubRelease> {
        let ver = if prerelease {
            let url = format!("{}/repos/phylum-dev/cli/releases", self.github_uri);

            self.get_github(url.as_str(), |r| async move {
                let data = r.json::<Vec<GithubRelease>>().await;

                match data {
                    Ok(data) => Some(data[0].to_owned()),
                    Err(error) => {
                        log::warn!("Failed latest version check: {:?}", error);
                        None
                    }
                }
            })
            .await
        } else {
            let url = format!("{}/repos/phylum-dev/cli/releases/latest", self.github_uri);

            self.get_github(url.as_str(), |r| async move {
                let data = r.json::<GithubRelease>().await;

                match data {
                    Ok(data) => Some(data),
                    Err(error) => {
                        log::warn!("Failed latest version check: {:?}", error);
                        None
                    }
                }
            })
            .await
        };

        log::debug!("Found latest version: {:?}", ver);

        ver
    }

    /// Download the binary specified in the Github release.
    ///
    /// On success, writes the requested file to the temporary system folder
    /// with the provided filename. Returns the path to the written file.
    async fn download_file(
        &self,
        latest: &GithubReleaseAsset,
        filename: &str,
    ) -> Result<String, std::io::Error> {
        let binary_path = self
            .get_github(latest.browser_download_url.as_str(), |r| async move {
                let dest = match tmp_path(filename) {
                    Some(path) => path,
                    None => return None,
                };

                let data = r.bytes().await.ok()?;

                let mut file =
                    std::fs::File::create(&dest).expect("Failed to create temporary update file");
                file.write_all(&data).expect("Failed to write update file");

                Option::Some::<String>(dest)
            })
            .await;

        match binary_path {
            Some(ret) => Ok(ret),
            _ => Err(std::io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to download {}", filename),
            )),
        }
    }

    /// Compare the current version as reported by Clap with the version currently
    /// published on Github. We do the naive thing here: If the latest version on
    /// Github does not match the Clap version, we indicate that we need to
    /// update. We do not compare semvers to determine if an update is required.
    pub fn needs_update(&self, current_version: &str, latest_version: &GithubRelease) -> bool {
        let latest = latest_version
            .name
            .replace("phylum ", "")
            .replace('v', "")
            .trim()
            .to_owned();
        let current = current_version.replace("phylum ", "").trim().to_owned();
        latest != current
    }

    /// Locate the specified asset in the Github response structure.
    pub fn find_github_asset<'a>(
        &self,
        latest: &'a GithubRelease,
        name: &str,
    ) -> Result<&'a GithubReleaseAsset, std::io::Error> {
        match latest.assets.iter().find(|x| x.name == name) {
            Some(x) => Ok(x),
            _ => Err(std::io::Error::new(
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
    pub async fn do_update(&self, latest: GithubRelease) -> Result<String, std::io::Error> {
        debug!("Performing the update process");
        let latest_version = &latest.name;

        let (bin_asset_name, shell_asset_name) = if cfg!(target_os = "macos") {
            ("phylum-macos-x86_64", "_phylum")
        } else if cfg!(target_os = "linux") {
            ("phylum-linux-x86_64", "phylum.bash")
        } else {
            return Err(std::io::Error::new(
                io::ErrorKind::Other,
                "The current OS is not currently supported for auto-update",
            ));
        };

        // Find location of assets on disk
        debug!("Locating the installed paths for the update");
        let installed_bin_path = self.installed_asset(None, "phylum")?;
        let prefix = if cfg!(target_os = "macos") {
            Some("completions")
        } else {
            None
        };
        let installed_bash_path = self.installed_asset(prefix, shell_asset_name)?;

        // Get the URL for each asset from the Github JSON response in `latest`.
        debug!("Finding the github assets in the Github JSON response");
        let bin_asset_url = self.find_github_asset(&latest, bin_asset_name)?;
        let bash_asset_url = self.find_github_asset(&latest, shell_asset_name)?;
        let minisign_name = format!("{}.minisig", bin_asset_name);
        let sig_asset_url = self.find_github_asset(&latest, minisign_name.as_str())?;

        debug!("Downloading the update files");
        let bin = self.download_file(bin_asset_url, "phylum.update").await?;
        let bash = self.download_file(bash_asset_url, shell_asset_name).await?;
        let sig = self
            .download_file(sig_asset_url, "phylum.update.minisig")
            .await?;

        debug!("Verifying the binary signature before move");
        if !self.has_valid_signature(bin.as_str(), sig.as_str()) {
            return Err(std::io::Error::new(
                io::ErrorKind::Other,
                "The update binary failed signature validation",
            ));
        }

        // If the download and validation succeeds _then_ we move it to overwrite
        // the existing binary and bash file.
        debug!("Copying the files to the intended install location");
        fs::remove_file(&installed_bin_path)?;
        fs::copy(&bin, &installed_bin_path)?;
        debug!(
            "Copying shell script from {} to {:?}",
            bash, installed_bash_path
        );
        fs::copy(&bash, &installed_bash_path)?;
        debug!("Setting permissions ");
        fs::set_permissions(&installed_bin_path, fs::Permissions::from_mode(0o770))?;
        debug!("Removing bin");
        fs::remove_file(&bin)?;
        debug!("Removing shell script");
        fs::remove_file(&bash)?;

        // Ensure that the files copied to the final location were the ones
        // we expected. This is to address a potential race condition between
        // the check and the copy.
        debug!("Verifying the file wasn't changed/tampered with before the move");
        let final_bin = match installed_bin_path.clone().into_os_string().into_string() {
            Ok(x) => x,
            Err(_) => {
                return Err(std::io::Error::new(
                    io::ErrorKind::Other,
                    "Could not create the path for the installation",
                ))
            }
        };

        if !self.has_valid_signature(final_bin.as_str(), sig.as_str()) {
            fs::remove_file(&installed_bin_path)?;
            fs::remove_file(&installed_bash_path)?;

            return Err(std::io::Error::new(
                io::ErrorKind::Other,
                "Possible attack attempt! Binary changed after initial signature verification and was removed.",
            ));
        }

        Ok(format!("Successfully updated to {}!", latest_version))
    }

    /// Verify that the downloaded binary matches the expected signature. Returns
    /// `true` for a valid signature, `false` otherwise.
    pub fn has_valid_signature(&self, file: &str, sig_path: &str) -> bool {
        let sig = fs::read_to_string(sig_path).expect("Unable to read signature file");
        let bin = fs::read(file).expect("Unable to read binary data from disk");

        let signature = match Signature::decode(&sig) {
            Ok(x) => x,
            Err(_) => return false,
        };

        self.pubkey.verify(&bin[..], &signature).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use crate::update::ApplicationUpdater;
    use minisign_verify::PublicKey;
    use std::fs;
    use std::fs::File;
    use std::io::prelude::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, ResponseTemplate};

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
    fn find_installed_asset_location() {
        let updater = ApplicationUpdater::default();
        let asset = updater.installed_asset(None, "example.ext").unwrap();
        assert!(asset.ends_with("example.ext"));
    }

    #[test]
    fn test_signature_validation() {
        let mut file = File::create("hello.txt").unwrap();
        let _ = file.write_all(b"Hello, world\n");

        let minisign_sig = b"untrusted comment: signature from minisign secret key\nRWT6G44ykbS8GJ+2A+Fjj6ZdR1/632p6WlwqAYhb8DSeKhCl3rzG1TGSF9CD9DDf9BdWrOjvnqi78yh38djVuYvAW2FhE0MvTQ4=\ntrusted comment: Phylum, Inc. - Future of software supply chain security\nkBL1siaOp2uZq2IrNKVguDGje88ghM2L0XJ6n/1rjGL2aQwbJ0fZPe5uOde3IbObPKTF4KCHbRtMALUEu6TaBQ==\n";

        let mut sig = File::create("hello.txt.minisig").unwrap();
        let _ = sig.write_all(minisign_sig);
        let updater = ApplicationUpdater::default();
        let valid = updater.has_valid_signature("hello.txt", "hello.txt.minisig");

        let _ = fs::remove_file("hello.txt");
        let _ = fs::remove_file("hello.txt.minisig");
        assert!(valid);
    }
}
