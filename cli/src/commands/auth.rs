use std::path::Path;

use anyhow::{anyhow, Context, Result};
use clap::ArgMatches;
use phylum_types::types::auth::RefreshToken;
use tokio::io::{self, AsyncBufReadExt, BufReader};

use crate::api::PhylumApi;
use crate::commands::{CommandResult, ExitCode};
use crate::config::{save_config, Config};
use crate::{auth, print_user_success, print_user_warning};

/// Register a user. Opens a browser, and redirects the user to the oauth server
/// registration page
async fn handle_auth_register(mut config: Config, config_path: &Path) -> Result<()> {
    let api_uri = &config.connection.uri;
    let ignore_certs = config.ignore_certs();
    config.auth_info = PhylumApi::register(config.auth_info, ignore_certs, api_uri).await?;
    save_config(config_path, &config).map_err(|error| anyhow!(error))?;
    Ok(())
}

/// Login a user. Opens a browser, and redirects the user to the oauth server
/// login page
async fn handle_auth_login(
    mut config: Config,
    config_path: &Path,
    matches: &ArgMatches,
) -> Result<()> {
    let api_uri = &config.connection.uri;
    let ignore_certs = config.ignore_certs();
    config.auth_info =
        PhylumApi::login(config.auth_info, ignore_certs, api_uri, matches.get_flag("reauth"))
            .await?;
    save_config(config_path, &config).map_err(|error| anyhow!(error))?;
    Ok(())
}

/// Display the current authentication status to the user.
pub async fn handle_auth_status(config: Config, timeout: Option<u64>) -> CommandResult {
    if config.auth_info.offline_access().is_none() {
        print_user_warning!("User is not currently authenticated");
        return Ok(ExitCode::NotAuthenticated);
    }

    // Create a client with our auth token attached.
    let api = PhylumApi::new(config, timeout).await?;

    let user_info = api.user_info().await;

    match user_info {
        Ok(user) => {
            print_user_success!(
                "Currently authenticated as '{}' with long lived refresh token",
                user.email
            );
            Ok(ExitCode::Ok)
        },
        Err(_err) => {
            print_user_warning!("Refresh token could not be validated");
            Ok(ExitCode::AuthenticationFailure)
        },
    }
}

/// Display the current authentication token to the user, if one exists.
pub async fn handle_auth_token(config: &Config, matches: &clap::ArgMatches) -> CommandResult {
    let refresh_token = match config.auth_info.offline_access() {
        Some(refresh_token) => refresh_token,
        None => {
            print_user_warning!(
                "User is not currently authenticated, please login with `phylum auth login`"
            );
            return Ok(ExitCode::NotAuthenticated);
        },
    };

    if matches.get_flag("bearer") {
        let api_uri = &config.connection.uri;
        let access_token =
            auth::handle_refresh_tokens(refresh_token, config.ignore_certs(), api_uri).await?;
        println!("{}", access_token);
        Ok(ExitCode::Ok)
    } else {
        println!("{refresh_token}");
        Ok(ExitCode::Ok)
    }
}

/// Read a non-empty line from stdin as the token
async fn stdin_read_token() -> Result<RefreshToken> {
    let mut reader = BufReader::new(io::stdin());
    let mut line = String::new();

    loop {
        if reader.read_line(&mut line).await? == 0 {
            return Err(anyhow!("unexpected EOF"));
        }

        match line.trim() {
            "" => {},
            line => return Ok(RefreshToken::new(line)),
        }
    }
}

/// Set the current authentication token.
pub async fn handle_auth_set_token(
    mut config: Config,
    matches: &clap::ArgMatches,
    config_path: &Path,
) -> CommandResult {
    let offline_access = match matches.get_one::<String>("token") {
        Some(t) => RefreshToken::new(t),
        None => stdin_read_token().await?,
    };
    config.auth_info.set_offline_access(offline_access);
    save_config(config_path, &config)?;
    Ok(ExitCode::Ok)
}

/// Handle the subcommands for the `auth` subcommand.
pub async fn handle_auth(
    config: Config,
    config_path: &Path,
    matches: &clap::ArgMatches,
    timeout: Option<u64>,
) -> CommandResult {
    match matches.subcommand() {
        Some(("register", _)) => match handle_auth_register(config, config_path).await {
            Ok(_) => {
                print_user_success!("{}", "User successfuly regsistered");
                Ok(ExitCode::Ok)
            },
            Err(error) => Err(error).context("User registration failed"),
        },
        Some(("login", matches)) => match handle_auth_login(config, config_path, matches).await {
            Ok(_) => {
                print_user_success!("{}", "User login successful");
                Ok(ExitCode::Ok)
            },
            Err(error) => Err(error).context("User login failed"),
        },
        Some(("status", _)) => handle_auth_status(config, timeout).await,
        Some(("token", matches)) => handle_auth_token(&config, matches).await,
        Some(("set-token", matches)) => handle_auth_set_token(config, matches, config_path).await,
        _ => unreachable!("invalid clap configuration"),
    }
}
