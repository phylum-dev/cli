use std::path::PathBuf;
use std::process;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::anyhow;
use clap::AppSettings;
use env_logger::Env;
use log::*;
use spinners::{Spinner, Spinners};

use phylum_cli::api::PhylumApi;
use phylum_cli::config::*;
use phylum_cli::update::ApplicationUpdater;
use phylum_types::types::common::JobId;
use phylum_types::types::job::Action;

mod commands;
mod print;
mod prompt;

use commands::auth::*;
use commands::jobs::*;
use commands::packages::*;
use commands::projects::handle_projects;
use commands::{CommandResult, CommandValue};
use print::*;

/// Exit with status code 0 and optionally print a message to the user.
pub fn exit_ok(message: Option<impl AsRef<str>>) -> ! {
    if let Some(message) = message {
        info!("{}", message.as_ref());
        print_user_success!("{}", message.as_ref());
    }
    process::exit(0)
}

/// Print a warning message to the user before exiting with exit code 0.
pub fn exit_warn(message: impl AsRef<str>) -> ! {
    warn!("{}", message.as_ref());
    print_user_warning!("Warning: {}", message.as_ref());
    process::exit(0)
}

/// Print an error to the user before exiting with exit code 1.
pub fn exit_fail(message: impl AsRef<str>) -> ! {
    error!("{}", message.as_ref());
    print_user_failure!("Error: {}", message.as_ref());
    process::exit(1)
}

/// Exit with status code 1, and optionally print a message to the user and
/// print error information.
pub fn exit_error(error: Box<dyn std::error::Error>, message: Option<impl AsRef<str>>) -> ! {
    match message {
        None => {
            error!("{}: {:?}", error, error);
            print_user_failure!("Error: {}", error);
        }
        Some(message) => {
            error!("{}: {:?}", message.as_ref(), error);
            print_user_failure!("Error: {} caused by: {}", message.as_ref(), error);
        }
    }
    process::exit(1)
}

async fn handle_commands() -> CommandResult {
    let app = phylum_cli::app::app()
        .setting(AppSettings::ArgRequiredElseHelp)
        .setting(AppSettings::SubcommandRequiredElseHelp);
    let app_name = app.get_name().to_string();
    let ver = app.get_version().unwrap();

    // Required for printing help messages since `get_matches()` consumes `App`
    let app_helper = &mut app.clone();

    let matches = app.get_matches();

    let settings_path = get_home_settings_path()?;

    let config_path = matches
        .value_of("config")
        .and_then(|config_path| shellexpand::env(config_path).ok())
        .map(|config_path| PathBuf::from(config_path.to_string()))
        .unwrap_or(settings_path);

    log::debug!("Reading config from {}", config_path.to_string_lossy());

    let mut config: Config = read_configuration(&config_path).map_err(|err| {
        anyhow!(
            "Failed to read configuration at `{}`: {}",
            config_path.to_string_lossy(),
            err
        )
    })?;

    let mut check_for_updates = false;

    if matches.subcommand_matches("update").is_none() {
        let start = SystemTime::now();
        let now = start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs() as usize;

        if let Some(last_update) = config.last_update {
            const SECS_IN_DAY: usize = 24 * 60 * 60;
            if now - last_update > SECS_IN_DAY {
                log::debug!("Checking for updates...");
                check_for_updates = true;
            }
        } else {
            check_for_updates = true;
        }

        if check_for_updates {
            config.last_update = Some(now);
            save_config(&config_path, &config)
                .unwrap_or_else(|e| log::error!("Failed to save config: {}", e));
        }
    }

    if check_for_updates {
        let updater = ApplicationUpdater::default();
        match updater.get_latest_version(false).await {
            Some(latest) => {
                if updater.needs_update(ver, &latest) {
                    print_update_message();
                }
            }
            None => log::debug!("Failed to get the latest version for update check"),
        }
    }

    // For these commands, we want to just provide verbose help and exit if no
    // arguments are supplied
    if let Some(matches) = matches.subcommand_matches("analyze") {
        if !matches.is_present("LOCKFILE") {
            print_sc_help(app_helper, "analyze");
            return Ok(CommandValue::Void);
        }
    } else if let Some(matches) = matches.subcommand_matches("package") {
        if !(matches.is_present("name") && matches.is_present("version")) {
            print_sc_help(app_helper, "package");
            return Ok(CommandValue::Void);
        }
    }

    if matches.subcommand_matches("version").is_some() {
        return CommandValue::String(format!("{app_name} (Version {ver})")).into();
    }

    let timeout = matches
        .value_of("timeout")
        .and_then(|t| t.parse::<u64>().ok());

    if let Some(matches) = matches.subcommand_matches("auth") {
        handle_auth(config, &config_path, matches, app_helper).await?;
        return CommandValue::Void.into();
    }

    let ignore_certs =
        matches.is_present("no-check-certificate") || config.ignore_certs.unwrap_or_default();
    if ignore_certs {
        log::warn!("Ignoring TLS server certificate verification per user request.");
    }

    let mut api = PhylumApi::new(
        &mut config.auth_info,
        &config.connection.uri,
        timeout,
        ignore_certs,
    )
    .await
    .map_err(|err| anyhow!("Error creating client").context(err))?;

    // PhylumApi may have had to log in, updating the auth info so we should save the config
    save_config(&config_path, &config).map_err(|error| {
        let msg = format!(
            "Failed to save configuration to '{}'",
            config_path.to_string_lossy()
        );
        anyhow!(msg).context(error)
    })?;

    if matches.subcommand_matches("ping").is_some() {
        let resp = api.ping().await;
        print_response(&resp, true, None);
        return CommandValue::Void.into();
    }

    let should_submit = matches.subcommand_matches("analyze").is_some()
        || matches.subcommand_matches("batch").is_some();

    // TODO this panicks with the type-checked `App` since the "cancel"
    // subcommand is undefined. Is the backend feature implemented, or
    // should we just keep this short circuited for now?
    let should_cancel = false;
    // let should_cancel = matches.subcommand_matches("cancel").is_some();

    // TODO: switch from if/else to non-exhaustive pattern match
    if let Some(matches) = matches.subcommand_matches("projects") {
        handle_projects(&mut api, matches).await?;
    } else if let Some(matches) = matches.subcommand_matches("update") {
        let spinner = Spinner::new(
            Spinners::Dots12,
            "Downloading update and verifying binary signatures...".into(),
        );
        let updater = ApplicationUpdater::default();
        match updater
            .get_latest_version(matches.is_present("prerelease"))
            .await
        {
            Some(ver) => match updater.do_update(ver).await {
                Ok(msg) => {
                    spinner.stop();
                    println!();
                    print_user_success!("{}", msg);
                }
                Err(msg) => {
                    spinner.stop();
                    println!();
                    print_user_failure!("{}", msg);
                }
            },
            _ => {
                spinner.stop();
                println!();
                print_user_warning!("Failed to get version metadata");
            }
        };
    } else if let Some(matches) = matches.subcommand_matches("package") {
        return handle_get_package(&mut api, &config.request_type, matches).await;
    } else if should_submit {
        return handle_submission(&mut api, config, &matches).await;
    } else if let Some(matches) = matches.subcommand_matches("history") {
        return handle_history(&mut api, matches).await;
    } else if should_cancel {
        if let Some(matches) = matches.subcommand_matches("cancel") {
            let request_id = matches.value_of("request_id").unwrap().to_string();
            let request_id = JobId::from_str(&request_id)
                .map_err(|err| anyhow!("Received invalid request id. Request id's should be of the form xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx").context(err))?;
            let resp = api.cancel(&request_id).await;
            print_response(&resp, true, None);
        }
    }

    CommandValue::Void.into()
}

#[tokio::main]
async fn main() {
    env_logger::from_env(Env::default().default_filter_or("warn")).init();

    match handle_commands().await {
        Ok(CommandValue::Action(action)) => match action {
            Action::None => exit_ok(None::<&str>),
            Action::Warn => exit_warn("Project failed threshold requirements!"),
            Action::Break => exit_fail("Project failed threshold requirements, failing the build!"),
        },
        Ok(CommandValue::String(message)) => exit_ok(Some(&message)),
        Ok(CommandValue::Void) => exit_ok(None::<&str>),
        Err(error) => {
            exit_error(error.into(), Some("Execution failed"));
        }
    }
}
