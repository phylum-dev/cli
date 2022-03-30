use std::path::Path;

use anyhow::anyhow;
use clap::Command;

use super::{CommandResult, CommandValue};
use crate::api::PhylumApi;
use crate::config::{save_config, Config};
use crate::print::print_sc_help;
use crate::print_user_success;
use crate::print_user_warning;

/// Register a user. Opens a browser, and redirects the user to the oauth server
/// registration page
async fn handle_auth_register(mut config: Config, config_path: &Path) -> CommandResult {
    config.auth_info = PhylumApi::register(config.auth_info).await?;
    save_config(config_path, &config).map_err(|error| anyhow!(error))?;
    CommandValue::Void.into()
}

/// Login a user. Opens a browser, and redirects the user to the oauth server
/// login page
async fn handle_auth_login(mut config: Config, config_path: &Path) -> CommandResult {
    config.auth_info = PhylumApi::login(config.auth_info).await?;
    save_config(config_path, &config).map_err(|error| anyhow!(error))?;
    CommandValue::Void.into()
}

/// Display the current authentication status to the user.
pub fn handle_auth_status(config: &Config) {
    if config.auth_info.offline_access.is_some() {
        print_user_success!("Currently authenticated with long lived refresh token");
    } else {
        print_user_warning!("User is not currently authenticated");
    }
}

/// Display the current authentication token to the user, if one exists.
pub fn handle_auth_token(config: &Config) {
    match config.auth_info.offline_access.clone() {
        Some(token) => {
            print_user_success!("{}", token);
        }
        None => {
            print_user_warning!(
                "User is not currently authenticated, please login with `phylum auth login`"
            );
        }
    };
}

/// Handle the subcommands for the `auth` subcommand.
pub async fn handle_auth(
    config: Config,
    config_path: &Path,
    matches: &clap::ArgMatches,
    app_helper: &mut Command<'_>,
) -> CommandResult {
    if matches.subcommand_matches("register").is_some() {
        match handle_auth_register(config, config_path).await {
            Ok(_) => {
                print_user_success!("{}", "User successfuly regsistered");
            }
            Err(error) => {
                return Err(anyhow!(
                    "User registration failed: {}",
                    error.root_cause().to_string()
                ))
            }
        }
    } else if matches.subcommand_matches("login").is_some() {
        match handle_auth_login(config, config_path).await {
            Ok(_) => {
                print_user_success!("{}", "User login successful");
            }
            Err(error) => {
                return Err(anyhow!(
                    "User login failed: {}",
                    error.root_cause().to_string()
                ));
            }
        }
    } else if matches.subcommand_matches("status").is_some() {
        handle_auth_status(&config);
    } else if matches.subcommand_matches("token").is_some() {
        handle_auth_token(&config);
    } else {
        print_sc_help(app_helper, "auth");
    }
    CommandValue::Void.into()
}
