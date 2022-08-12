use std::fmt::Display;
use std::path::PathBuf;
use std::time::UNIX_EPOCH;

use anyhow::{anyhow, Context, Result};
use clap::ArgMatches;
use env_logger::Env;
use phylum_cli::api::PhylumApi;
#[cfg(feature = "selfmanage")]
use phylum_cli::commands::uninstall;
use phylum_cli::commands::{
    auth, extensions, group, jobs, packages, parse, project, CommandResult, CommandValue, ExitCode,
};
use phylum_cli::config::{self, Config};
use phylum_cli::spinner::Spinner;
use phylum_cli::{print, print_user_failure, print_user_success, print_user_warning, update};
use phylum_types::types::job::Action;

/// Print a warning message to the user before exiting with exit code 0.
pub fn exit_warn(message: impl AsRef<str>) -> ! {
    print_user_warning!("Warning: {}", message.as_ref());
    ExitCode::Ok.exit()
}

/// Print an error to the user before exiting with the passed exit code.
pub fn exit_fail(message: impl Display, exit_code: ExitCode) -> ! {
    print_user_failure!("Error: {}", message);
    exit_code.exit()
}

/// Construct an instance of `PhylumApi` given configuration, optional timeout,
/// and whether we need API to ignore certificates.
async fn api_factory(
    config: Config,
    config_path: PathBuf,
    timeout: Option<u64>,
) -> Result<PhylumApi> {
    let api = PhylumApi::new(config, timeout).await?;

    // PhylumApi may have had to log in, updating the auth info so we should save
    // the config
    config::save_config(&config_path, api.config()).with_context(|| {
        format!("Failed to save configuration to '{}'", config_path.to_string_lossy())
    })?;

    Ok(api)
}

async fn handle_commands() -> CommandResult {
    // Initialize clap app and read configuration.
    //

    let app = phylum_cli::app::app().arg_required_else_help(true).subcommand_required(true);
    let app_name = app.get_name().to_string();
    // Required for printing help messages since `get_matches()` consumes `App`
    let app_helper = &mut app.clone();
    let ver = app.get_version().unwrap();
    let matches = app.get_matches();

    let settings_path = config::get_home_settings_path()?;
    let config_path = matches
        .value_of("config")
        .and_then(|config_path| shellexpand::env(config_path).ok())
        .map(|config_path| PathBuf::from(config_path.to_string()))
        .unwrap_or(settings_path);

    log::debug!("Reading config from {}", config_path.to_string_lossy());
    let mut config: Config = config::read_configuration(&config_path).map_err(|err| {
        anyhow!("Failed to read configuration at `{}`: {}", config_path.to_string_lossy(), err)
    })?;
    config.ignore_certs |= matches.is_present("no-check-certificate");

    if config.ignore_certs {
        log::warn!("Ignoring TLS server certificate verification per user request.");
    }

    // We initialize these value here, for later use by the PhylumApi object.
    let timeout = matches.value_of("timeout").and_then(|t| t.parse::<u64>().ok());

    // Check for updates, if we haven't explicitly invoked `update`.
    //

    if matches.subcommand_matches("update").is_none() {
        let now = UNIX_EPOCH.elapsed().expect("Time went backwards").as_secs() as usize;

        let check_for_updates = config.last_update.map_or(true, |last_update| {
            const SECS_IN_DAY: usize = 24 * 60 * 60;
            now - last_update > SECS_IN_DAY
        });

        if check_for_updates {
            log::debug!("Checking for updates...");

            // Update last update check timestamp.
            config.last_update = Some(now);
            config::save_config(&config_path, &config)
                .unwrap_or_else(|e| log::error!("Failed to save config: {}", e));

            if update::needs_update(false).await {
                print::print_update_message();
            }
        }
    }

    // Subcommands with precedence
    //

    // For these commands, we want to just provide verbose help and exit if no
    // arguments are supplied.
    if let Some(matches) = matches.subcommand_matches("analyze") {
        if !matches.is_present("LOCKFILE") {
            print::print_sc_help(app_helper, "analyze");
            return Ok(ExitCode::Ok.into());
        }
    } else if let Some(matches) = matches.subcommand_matches("package") {
        if !(matches.is_present("name") && matches.is_present("version")) {
            print::print_sc_help(app_helper, "package");
            return Ok(ExitCode::Ok.into());
        }
    }

    // Get the future, but don't await. Commands that require access to the API will
    // await on this, so that the API is not instantiated ahead of time for
    // subcommands that don't require it.
    let api = api_factory(config.clone(), config_path.clone(), timeout);

    let (subcommand, sub_matches) = matches.subcommand().unwrap();
    match subcommand {
        "auth" => {
            drop(api);
            auth::handle_auth(config, &config_path, sub_matches, app_helper, timeout).await
        },
        "version" => handle_version(&app_name, ver),
        "update" => handle_update(sub_matches).await,
        "parse" => parse::handle_parse(sub_matches),
        "ping" => handle_ping(Spinner::wrap(api).await?).await,
        "project" => project::handle_project(&mut Spinner::wrap(api).await?, sub_matches).await,
        "package" => {
            packages::handle_get_package(&mut Spinner::wrap(api).await?, sub_matches).await
        },
        "history" => jobs::handle_history(&mut Spinner::wrap(api).await?, sub_matches).await,
        "group" => group::handle_group(&mut Spinner::wrap(api).await?, sub_matches).await,
        "analyze" | "batch" => {
            jobs::handle_submission(&mut Spinner::wrap(api).await?, &matches).await
        },

        #[cfg(feature = "selfmanage")]
        "uninstall" => uninstall::handle_uninstall(sub_matches),

        "extension" => extensions::handle_extensions(Box::pin(api), sub_matches, app_helper).await,
        extension_subcmd => {
            extensions::handle_run_extension(Box::pin(api), extension_subcmd, sub_matches).await
        },
    }
}

async fn handle_ping(api: PhylumApi) -> CommandResult {
    print_user_success!("{}", api.ping().await?);
    Ok(ExitCode::Ok.into())
}

async fn handle_update(matches: &ArgMatches) -> CommandResult {
    let res = update::do_update(matches.is_present("prerelease")).await;
    let message = res?;
    print_user_success!("{}", message);
    Ok(ExitCode::Ok.into())
}

fn handle_version(app_name: &str, ver: &str) -> CommandResult {
    print_user_success!("{app_name} (Version {ver})");
    Ok(ExitCode::Ok.into())
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("warn")).init();

    match handle_commands().await {
        Ok(CommandValue::Action(action)) => match action {
            Action::None => ExitCode::Ok.exit(),
            Action::Warn => exit_warn("Project failed threshold requirements!"),
            Action::Break => exit_fail(
                "Project failed threshold requirements, failing the build!",
                ExitCode::FailedThresholds,
            ),
        },
        Ok(CommandValue::Code(code)) => code.exit(),
        Err(error) => exit_fail(format!("{:?}", error), ExitCode::Generic),
    }
}
