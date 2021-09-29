use std::process;
use std::str::FromStr;

use ansi_term::Color::{Blue, Green};
use anyhow::Result;
use clap::App;
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Input, Password};

use phylum_cli::api::PhylumApi;
use phylum_cli::auth::*;
use phylum_cli::config::{save_config, Config};
use phylum_cli::types::Key;

use crate::exit::exit_error;
use crate::print::print_response;
use crate::print_sc_help;
use crate::print_user_failure;
use crate::print_user_success;
use crate::print_user_warning;

/// Register a user. Drops the user into an interactive mode to get the user's
/// details.
fn handle_auth_register(
    api: &mut PhylumApi,
    config: &mut Config,
    config_path: &str,
) -> Result<String, std::io::Error> {
    let name: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Your name")
        .interact_text()?;

    let email: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Email address")
        .validate_with({
            move |email: &String| -> Result<(), &str> {
                // Naive check for email. Additional validation should
                // occur on the backend.
                match email.contains('@') && email.contains('.') {
                    true => Ok(()),
                    false => Err("That is not a valid email address"),
                }
            }
        })
        .interact_text()?;

    let password: String = Password::with_theme(&ColorfulTheme::default())
        .with_prompt("Password")
        .with_confirmation("Confirm password", "Passwords do not match")
        .interact()?;

    api.register(email.as_str(), password.as_str(), name.as_str())
        .unwrap_or_else(|err| {
            exit_error(err, Some("Error registering user"));
        });

    config.auth_info.user = email;
    config.auth_info.pass = password;
    save_config(config_path, &config).unwrap_or_else(|err| {
        log::error!("Failed to save user credentials to config: {}", err);
        print_user_failure!("Failed to save user credentials: {}", err);
    });

    Ok("Successfully registered a new account!".to_string())
}

/// Authenticate a user with email and password.
///
/// Drops the user into an interactive mode to retrieve this information. If
/// authentication succeeds, persists the data to the configuration file. On
/// failure, returns a non-zero exit code.
fn handle_auth_login(
    api: &mut PhylumApi,
    config: &mut Config,
    config_path: &str,
) -> Result<String, std::io::Error> {
    let email: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Email address")
        .validate_with({
            move |email: &String| -> Result<(), &str> {
                // Naive check for email. Additional validation should
                // occur on the backend.
                match email.contains('@') && email.contains('.') {
                    true => Ok(()),
                    false => Err("That is not a valid email address"),
                }
            }
        })
        .interact_text()?;

    let password: String = Password::with_theme(&ColorfulTheme::default())
        .with_prompt("Password")
        .interact()?;

    // First login with the provided credentials. If the login is successful,
    // save the authentication information in our settings file.
    api.authenticate(&email, &password).unwrap_or_else(|err| {
        exit_error(err, None::<&str>);
    });

    config.auth_info.user = email;
    config.auth_info.pass = password;
    config.auth_info.api_token = None;
    save_config(config_path, &config).unwrap_or_else(|err| {
        log::error!("Failed to save user credentials to config: {}", err);
        print_user_failure!("{}", err);
    });

    Ok("Successfully authenticated with Phylum".to_string())
}

/// Handles the management of API keys.
///
/// Provides subcommands for:
///
/// * Creating a new key with `create`
/// * Deactivating a key with `remove`
/// * Listing all active keys with `list`
fn handle_auth_keys(
    api: &mut PhylumApi,
    config: &mut Config,
    config_path: &str,
    matches: &clap::ArgMatches,
) {
    if matches.subcommand_matches("create").is_some() {
        let resp = api.create_api_token();
        log::info!("==> Token created: `{:?}`", resp);
        if let Ok(ref resp) = resp {
            config.auth_info.api_token = Some(resp.to_owned());
            save_config(config_path, &config)
                .unwrap_or_else(|err| log::error!("Failed to save api token to config: {}", err));

            let key: String = resp.key.to_string();
            print_user_success!(
                "Successfully created new API key: \n\t{}\n",
                Green.paint(key)
            );
            return;
        }
    } else if let Some(action) = matches.subcommand_matches("remove") {
        let token_id = action.value_of("key_id").unwrap();
        let token = Key::from_str(token_id)
            .unwrap_or_else(|err| exit_error(err, Some("Received invalid token id")));
        let resp = api.delete_api_token(&token);
        log::info!("==> {:?}", resp);
        config.auth_info.api_token = None;
        save_config(config_path, &config)
            .unwrap_or_else(|err| log::error!("Failed to clear api token from config: {}", err));
        print_user_success!("Successfully deleted API key");
    } else if matches.subcommand_matches("list").is_some() || matches.subcommand().is_none() {
        let resp = api.get_api_tokens();

        // We only show the user the active API keys.
        let keys: Vec<ApiToken> = resp
            .unwrap_or_default()
            .into_iter()
            .filter(|k| k.active)
            .collect();

        if keys.is_empty() {
            print_user_success!(
                "No API keys available. Create your first key:\n\n\t{}\n",
                Blue.paint("phylum auth keys create")
            );
            return;
        }

        println!(
            "\n{:<35} | {}",
            Blue.paint("Created").to_string(),
            Blue.paint("API Key").to_string()
        );

        let res = Ok(keys);
        println!("{:-^65}", "");
        print_response(&res, true, None);
        println!();
    }
}

pub fn authenticate(
    api: &mut PhylumApi,
    config: &mut Config,
    should_manage_tokens: bool,
) -> Result<(), phylum_cli::restson::Error> {
    log::debug!("Authenticating...");
    log::debug!("Auth config:\n{:?}", config.auth_info);

    // If an API token has been configured, prefer that.  Otherwise, log in with
    //  a standard username and password to get a JWT.
    if !should_manage_tokens {
        // auth endpoint doesn't support token auth
        if let Some(ref token) = config.auth_info.api_token {
            log::debug!("using token auth");
            api.set_api_token(token).unwrap_or_else(|err| {
                log::error!("Failed to set API token: {}", err);
            });
        }
    }

    if api.offline_access.is_none() {
        log::debug!("using standard auth");
        let resp = api
            .authenticate(&config.auth_info.user, &config.auth_info.pass)
            .map(|_t| ());
        log::debug!("==> {:?}", resp);
        return resp;
    }

    Ok(())
}

/// Display the current authentication status to the user.
pub fn handle_auth_status(api: &mut PhylumApi, config: &mut Config) {
    let resp = authenticate(api, config, false);

    if resp.is_ok() {
        if let Ok(true) = api.auth_status() {
            if config.auth_info.api_token.is_some() {
                let key = config.auth_info.api_token.as_ref().unwrap().key.to_string();
                print_user_success!("Currently authenticated with API key {}", Green.paint(key));
            } else if !config.auth_info.user.is_empty() {
                print_user_success!(
                    "Currently authenticated as {}",
                    Green.paint(&config.auth_info.user)
                );
            }
            return;
        }
    }

    print_user_warning!("User is not currently authenticated");
}

/// Handle the subcommands for the `auth` subcommand.
pub fn handle_auth(
    api: &mut PhylumApi,
    config: &mut Config,
    config_path: &str,
    matches: &clap::ArgMatches,
    app_helper: &mut App,
) {
    if matches.subcommand_matches("register").is_some() {
        match handle_auth_register(api, config, config_path) {
            Ok(msg) => {
                print_user_success!("{}", msg);
            }
            Err(msg) => {
                print_user_failure!("{}", msg);
                process::exit(-1);
            }
        }
    } else if matches.subcommand_matches("login").is_some() {
        match handle_auth_login(api, config, config_path) {
            Ok(msg) => {
                print_user_success!("{}", msg);
            }
            Err(msg) => {
                print_user_failure!("{}", msg);
                process::exit(-1);
            }
        }
    } else if let Some(subcommand) = matches.subcommand_matches("keys") {
        handle_auth_keys(api, config, config_path, subcommand);
    } else if matches.subcommand_matches("status").is_some() {
        handle_auth_status(api, config);
    } else {
        print_sc_help(app_helper, "auth");
    }
}
