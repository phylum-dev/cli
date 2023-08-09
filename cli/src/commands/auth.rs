use std::borrow::Cow;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use clap::ArgMatches;
use dialoguer::MultiSelect;
use phylum_types::types::auth::RefreshToken;
use tokio::io::{self, AsyncBufReadExt, BufReader};

use crate::api::PhylumApi;
use crate::auth::is_locksmith_token;
use crate::commands::{CommandResult, ExitCode};
use crate::config::{save_config, Config};
use crate::format::Format;
use crate::{auth, print_user_failure, print_user_success, print_user_warning};

/// Register a user. Opens a browser, and redirects the user to the oauth server
/// registration page
async fn handle_auth_register(
    mut config: Config,
    config_path: &Path,
    matches: &ArgMatches,
) -> Result<()> {
    let api_uri = &config.connection.uri;
    let ignore_certs = config.ignore_certs();
    config.auth_info = PhylumApi::register(
        config.auth_info,
        matches.get_one("token-name").cloned(),
        ignore_certs,
        api_uri,
    )
    .await?;
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
    config.auth_info = PhylumApi::login(
        config.auth_info,
        matches.get_one("token-name").cloned(),
        ignore_certs,
        api_uri,
        matches.get_flag("reauth"),
    )
    .await?;
    save_config(config_path, &config).map_err(|error| anyhow!(error))?;
    Ok(())
}

/// Display the current authentication status to the user.
pub async fn handle_auth_status(config: Config, timeout: Option<u64>) -> CommandResult {
    let auth_type = match config.auth_info.offline_access() {
        Some(token) if is_locksmith_token(token) => "API key",
        Some(_) => "OpenID Connect",
        None => {
            print_user_warning!("User is not currently authenticated");
            return Ok(ExitCode::NotAuthenticated);
        },
    };

    // Create a client with our auth token attached.
    let api = PhylumApi::new(config, timeout).await?;

    let user_info = api.user_info().await;

    match user_info {
        Ok(user) => {
            print_user_success!(
                "Currently authenticated as '{}' via {}",
                user.identity(),
                auth_type
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
            auth::renew_access_token(refresh_token, config.ignore_certs(), api_uri).await?;
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

/// List all tokens associated with the logged-in user.
pub async fn handle_auth_list_tokens(
    config: Config,
    matches: &clap::ArgMatches,
    timeout: Option<u64>,
) -> CommandResult {
    // Create a client with our auth token attached.
    let api = PhylumApi::new(config, timeout).await?;

    let tokens = api.list_tokens().await?;

    let pretty_print = !matches.get_flag("json");
    tokens.write_stdout(pretty_print);

    Ok(ExitCode::Ok)
}

/// Revoke the specified authentication token.
pub async fn handle_auth_revoke_token(
    config: Config,
    matches: &clap::ArgMatches,
    timeout: Option<u64>,
) -> CommandResult {
    // Create a client with our auth token attached.
    let api = PhylumApi::new(config, timeout).await?;

    // If no name is provided, we show a simple selection UI.
    let names = match matches.get_many::<String>("token-name") {
        Some(names) => names.into_iter().map(Cow::Borrowed).collect(),
        None => {
            // Get all available tokens from Locksmith API.
            let tokens = api.list_tokens().await?;
            let mut token_names = tokens.into_iter().map(|token| token.name).collect::<Vec<_>>();

            // Prompt user to select all tokens.
            let prompt = "[SPACE] Select  [ENTER] Confirm\nAPI tokens which will be revoked";
            let indices = MultiSelect::new().with_prompt(prompt).items(&token_names).interact()?;

            // Get names for all selected tokens.
            let tokens = indices
                .into_iter()
                .rev()
                .map(|index| Cow::Owned(token_names.swap_remove(index)))
                .collect::<Vec<_>>();

            tokens
        },
    };

    println!();

    // Indicate to user why no action was taken.
    if names.is_empty() {
        print_user_warning!("Skipping revocation: No token selected");
    }

    // Revoke all selected tokens.
    for name in names {
        match api.revoke_token(&name).await {
            Ok(()) => print_user_success!("Successfully revoked token {name:?}"),
            Err(err) => print_user_failure!("Could not revoke token {name:?}: {err}"),
        }
    }

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
        Some(("register", _)) => match handle_auth_register(config, config_path, matches).await {
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
        Some(("list-tokens", matches)) => handle_auth_list_tokens(config, matches, timeout).await,
        Some(("revoke-token", matches)) => handle_auth_revoke_token(config, matches, timeout).await,
        _ => unreachable!("invalid clap configuration"),
    }
}
