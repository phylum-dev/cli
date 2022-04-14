use std::path::PathBuf;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context};
use env_logger::Env;
use log::*;
use phylum_cli::commands::parse::handle_parse;
use spinners::{Spinner, Spinners};

use phylum_cli::api::PhylumApi;
use phylum_cli::commands::auth::*;
use phylum_cli::commands::group::handle_group;
use phylum_cli::commands::jobs::*;
use phylum_cli::commands::packages::*;
use phylum_cli::commands::project::handle_project;
#[cfg(feature = "selfmanage")]
use phylum_cli::commands::uninstall::*;
use phylum_cli::commands::{CommandResult, CommandValue, ExitCode};
use phylum_cli::config::*;
use phylum_cli::print::*;
use phylum_cli::update;
use phylum_cli::{print_user_failure, print_user_success, print_user_warning};
use phylum_types::types::job::Action;

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
pub fn exit_error(error: Box<dyn std::error::Error>, message: impl AsRef<str>) -> ! {
    error!("{}: {:?}", message.as_ref(), error);
    print_user_failure!("Error: {} caused by: {}", message.as_ref(), error);
    process::exit(1)
}

async fn handle_commands() -> CommandResult {
    let app = phylum_cli::app::app()
        .arg_required_else_help(true)
        .subcommand_required(true);
    let app_name = app.get_name().to_string();
    let ver = app.get_version().unwrap();

    // Required for printing help messages since `get_matches()` consumes `App`
    let app_helper = &mut app.clone();

    let matches = app.get_matches();

    #[cfg(feature = "selfmanage")]
    if let Some(matches) = matches.subcommand_matches("uninstall") {
        return handle_uninstall(matches);
    }

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

    if check_for_updates && update::needs_update(false).await {
        print_update_message();
    }

    // For these commands, we want to just provide verbose help and exit if no
    // arguments are supplied
    if let Some(matches) = matches.subcommand_matches("analyze") {
        if !matches.is_present("LOCKFILE") {
            print_sc_help(app_helper, "analyze");
            return Ok(ExitCode::Ok.into());
        }
    } else if let Some(matches) = matches.subcommand_matches("package") {
        if !(matches.is_present("name") && matches.is_present("version")) {
            print_sc_help(app_helper, "package");
            return Ok(ExitCode::Ok.into());
        }
    }

    if matches.subcommand_matches("version").is_some() {
        print_user_success!("{app_name} (Version {ver})");
        return Ok(ExitCode::Ok.into());
    }

    if let Some(matches) = matches.subcommand_matches("update") {
        let mut spinner = Spinner::new(
            Spinners::Dots12,
            "Downloading update and verifying binary signatures...".into(),
        );
        let res = update::do_update(matches.is_present("prerelease")).await;
        spinner.stop_with_newline();
        let message = res?;
        print_user_success!("{}", message);
        return Ok(ExitCode::Ok.into());
    }

    if let Some(matches) = matches.subcommand_matches("parse") {
        return handle_parse(matches);
    }

    let timeout = matches
        .value_of("timeout")
        .and_then(|t| t.parse::<u64>().ok());

    let ignore_certs =
        matches.is_present("no-check-certificate") || config.ignore_certs.unwrap_or_default();
    if ignore_certs {
        log::warn!("Ignoring TLS server certificate verification per user request.");
    }

    if let Some(matches) = matches.subcommand_matches("auth") {
        return handle_auth(
            config,
            &config_path,
            matches,
            app_helper,
            timeout,
            ignore_certs,
        )
        .await;
    }

    let mut api = PhylumApi::new(
        &mut config.auth_info,
        &config.connection.uri,
        timeout,
        ignore_certs,
    )
    .await
    .context("Error creating client")?;

    // PhylumApi may have had to log in, updating the auth info so we should save the config
    save_config(&config_path, &config).with_context(|| {
        format!(
            "Failed to save configuration to '{}'",
            config_path.to_string_lossy()
        )
    })?;

    if matches.subcommand_matches("ping").is_some() {
        let resp = api.ping().await;
        print_response(&resp, true, None);
        return Ok(ExitCode::Ok.into());
    }

    let should_submit = matches.subcommand_matches("analyze").is_some()
        || matches.subcommand_matches("batch").is_some();

    // TODO: switch from if/else to non-exhaustive pattern match
    if let Some(matches) = matches.subcommand_matches("project") {
        handle_project(&mut api, matches).await?;
    } else if let Some(matches) = matches.subcommand_matches("package") {
        return handle_get_package(&mut api, &config.request_type, matches).await;
    } else if should_submit {
        return handle_submission(&mut api, config, &matches).await;
    } else if let Some(matches) = matches.subcommand_matches("history") {
        return handle_history(&mut api, matches).await;
    } else if let Some(matches) = matches.subcommand_matches("group") {
        return handle_group(&mut api, matches).await;
    }

    Ok(ExitCode::Ok.into())
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("warn")).init();

    match handle_commands().await {
        Ok(CommandValue::Action(action)) => match action {
            Action::None => process::exit(0),
            Action::Warn => exit_warn("Project failed threshold requirements!"),
            Action::Break => exit_fail("Project failed threshold requirements, failing the build!"),
        },
        Ok(CommandValue::Code(code)) => process::exit(code as i32),
        Err(error) => exit_error(error.into(), "Execution failed"),
    }
}
