use phylum_cli::config::{AuthInfo, Config, ConnectionInfo};
use phylum_types::types::auth::RefreshToken;

use crate::common::{TestCli, API_URL};

#[test]
fn pass_api_key_through_env() {
    const ENV_TOKEN: &str = "ENV VARIABLE TOKEN";

    TestCli::builder()
        .cwd_temp()
        .build()
        .cmd()
        .env("PHYLUM_API_KEY", ENV_TOKEN)
        .args(&["auth", "token"])
        .assert()
        .success()
        .stdout(format!("{ENV_TOKEN}\n"));
}

#[test]
fn ignore_empty_token() {
    const CONFIG_TOKEN: &str = "CONFIGTOKEN";

    let config = Config {
        connection: ConnectionInfo { uri: API_URL.into() },
        auth_info: AuthInfo::new(Some(RefreshToken::new(CONFIG_TOKEN))),
        ..Config::default()
    };

    TestCli::builder()
        .with_config(config)
        .cwd_temp()
        .build()
        .cmd()
        .env("PHYLUM_API_KEY", "")
        .args(&["auth", "token"])
        .assert()
        .success()
        .stdout(format!("{CONFIG_TOKEN}\n"));
}
