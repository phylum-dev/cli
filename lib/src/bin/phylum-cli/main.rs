use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::anyhow;
use clap::{load_yaml, App, AppSettings};
use env_logger::Env;
use home::home_dir;
use spinners::{Spinner, Spinners};

use phylum_cli::api::PhylumApi;
use phylum_cli::config::*;
use phylum_cli::types::*;
use phylum_cli::update::ApplicationUpdater;

mod commands;
mod exit;
mod print;
mod prompt;

use commands::auth::*;
use commands::jobs::*;
use commands::packages::*;
use commands::{CommandResult, CommandValue};
use exit::*;
use print::*;

use crate::commands::projects::handle_projects;

async fn handle_commands() -> CommandResult {
    let yml = load_yaml!("../.conf/cli.yaml");
    let app = App::from(yml)
        .setting(AppSettings::ArgRequiredElseHelp)
        .setting(AppSettings::SubcommandRequiredElseHelp);
    let ver = &app.render_version();

    // Required for printing help messages since `get_matches()` consumes `App`
    let app_helper = &mut app.clone();

    let matches = app.get_matches();

    let home_path = home_dir().ok_or_else(|| anyhow!("Couldn't find the user's home directory"))?;

    let settings_path = home_path.as_path().join(".phylum").join("settings.yaml");

    let settings_path = settings_path.to_str().ok_or_else(|| {
        log::error!("Unicode parsing error in configuration file path");
        anyhow!(
            "Unable to read path to configuration file at, invalud unicode '{:?}'",
            home_path
        )
    })?;

    let config_path = matches.value_of("config").unwrap_or(settings_path);

    log::debug!("Reading config from {}", config_path);

    let mut config: Config = read_configuration(config_path)
        .map_err(|err| anyhow!("Failed to read configuration at `{}`: {}", config_path, err))?;

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
            save_config(config_path, &config)
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
        let name = yml["name"].as_str().unwrap_or("");
        let version = yml["version"].as_str().unwrap_or("");

        return CommandValue::String(format!("{} (Version {})", name, version)).into();
    }

    let timeout = matches
        .value_of("timeout")
        .and_then(|t| t.parse::<u64>().ok());

    if let Some(matches) = matches.subcommand_matches("auth") {
        handle_auth(config, config_path, matches, app_helper).await?;
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
    save_config(config_path, &config).map_err(|error| {
        let msg = format!("Failed to save configuration to '{}'", config_path);
        anyhow!(msg).context(error)
    })?;

    if matches.subcommand_matches("ping").is_some() {
        let resp = api.ping().await;
        print_response(&resp, true, None);
        return CommandValue::Void.into();
    }

    let should_submit = matches.subcommand_matches("analyze").is_some()
        || matches.subcommand_matches("batch").is_some();
    let should_cancel = matches.subcommand_matches("cancel").is_some();

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
        Ok(CommandValue::String(message)) => println!("{}", message),
        Ok(CommandValue::Action(action)) => match action {
            Action::None => exit_ok(None::<&str>),
            Action::Warn => exit_warn("Project failed threshold requirements!"),
            Action::Break => exit_fail("Project failed threshold requirements, failing the build!"),
        },
        Ok(CommandValue::Void) => {
            //nop
        }
        Err(error) => {
            log::error!("Execution failed, cause: {:?}", error);
            exit_error(error.into(), Some("Execution failed"));
        }
    }
}
