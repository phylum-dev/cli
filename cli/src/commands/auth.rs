use std::path::Path;

use anyhow::{anyhow, Context, Result};
use clap::Command;

use crate::api::PhylumApi;
use crate::commands::{CommandResult, ExitCode};
use crate::config::{save_config, Config};
use crate::print::print_sc_help;
use crate::print_user_success;
use crate::print_user_warning;

/// Register a user. Opens a browser, and redirects the user to the oauth server
/// registration page
async fn handle_auth_register(mut config: Config, config_path: &Path) -> Result<()> {
    config.auth_info = PhylumApi::register(config.auth_info).await?;
    save_config(config_path, &config).map_err(|error| anyhow!(error))?;
    Ok(())
}

/// Login a user. Opens a browser, and redirects the user to the oauth server
/// login page
async fn handle_auth_login(mut config: Config, config_path: &Path) -> Result<()> {
    config.auth_info = PhylumApi::login(config.auth_info).await?;
    save_config(config_path, &config).map_err(|error| anyhow!(error))?;
    Ok(())
}

/// Display the current authentication status to the user.
pub async fn handle_auth_status(
    mut config: Config,
    timeout: Option<u64>,
    ignore_certs: bool,
) -> CommandResult {
    if config.auth_info.offline_access.is_none() {
        print_user_warning!("User is not currently authenticated");
        return Ok(ExitCode::NotAuthenticated.into());
    }

    // Create a client with our auth token attached.
    let api = PhylumApi::new(
        &mut config.auth_info,
        &config.connection.uri,
        timeout,
        ignore_certs,
    )
    .await
    .context("Error creating client")?;

    let user_info = api.user_info(&config.auth_info).await;

    match user_info {
        Ok(user) => {
            print_user_success!(
                "Currently authenticated as '{}' with long lived refresh token",
                user.email
            );
            Ok(ExitCode::Ok.into())
        }
        Err(_err) => {
            print_user_warning!("Refresh token could not be validated");
            Ok(ExitCode::AuthenticationFailure.into())
        }
    }
}

/// Display the current authentication token to the user, if one exists.
pub fn handle_auth_token(config: &Config) -> CommandResult {
    match config.auth_info.offline_access.clone() {
        Some(token) => {
            println!("{}", token);
            Ok(ExitCode::Ok.into())
        }
        None => {
            print_user_warning!(
                "User is not currently authenticated, please login with `phylum auth login`"
            );
            Ok(ExitCode::NotAuthenticated.into())
        }
    }
}

/// Handle the subcommands for the `auth` subcommand.
pub async fn handle_auth(
    config: Config,
    config_path: &Path,
    matches: &clap::ArgMatches,
    app_helper: &mut Command<'_>,
    timeout: Option<u64>,
    ignore_certs: bool,
) -> CommandResult {
    if matches.subcommand_matches("register").is_some() {
        match handle_auth_register(config, config_path).await {
            Ok(_) => {
                print_user_success!("{}", "User successfuly regsistered");
                Ok(ExitCode::Ok.into())
            }
            Err(error) => Err(anyhow!(
                "User registration failed: {}",
                error.root_cause().to_string()
            )),
        }
    } else if matches.subcommand_matches("login").is_some() {
        match handle_auth_login(config, config_path).await {
            Ok(_) => {
                print_user_success!("{}", "User login successful");
                Ok(ExitCode::Ok.into())
            }
            Err(error) => Err(anyhow!(
                "User login failed: {}",
                error.root_cause().to_string()
            )),
        }
    } else if matches.subcommand_matches("status").is_some() {
        handle_auth_status(config, timeout, ignore_certs).await
    } else if matches.subcommand_matches("token").is_some() {
        handle_auth_token(&config)
    } else {
        print_sc_help(app_helper, "auth");
        Ok(ExitCode::Ok.into())
    }
}
