use std::fmt::Display;
use std::time::UNIX_EPOCH;

use anyhow::{Context, Result};
#[cfg(feature = "selfmanage")]
use clap::ArgMatches;
use env_logger::Env;
use log::LevelFilter;
use phylum_cli::api::PhylumApi;
#[cfg(unix)]
use phylum_cli::commands::sandbox;
#[cfg(feature = "selfmanage")]
use phylum_cli::commands::uninstall;
use phylum_cli::commands::{
    auth, extensions, find_dependency_files, group, init, jobs, packages, parse, project, status,
    CommandResult, ExitCode,
};
use phylum_cli::config::{self, Config};
use phylum_cli::spinner::Spinner;
use phylum_cli::{print, print_user_failure, print_user_success, update};

const LICENSE_BLURB: &str = r#"
Copyright (C) 2022  Phylum, Inc.
License GPLv3+: GNU GPL version 3 or later <https://gnu.org/licenses/gpl.html>
This is free software: you are free to change and redistribute it.
There is NO WARRANTY, to the extent permitted by law."#;

/// Print an error to the user before exiting with the passed exit code.
fn exit_fail(message: impl Display, exit_code: ExitCode) -> ! {
    print_user_failure!("Error: {}", message);
    exit_code.exit()
}

/// Construct an instance of `PhylumApi` given configuration, optional timeout,
/// and whether we need API to ignore certificates.
async fn api_factory(config: Config, timeout: Option<u64>) -> Result<PhylumApi> {
    let api = PhylumApi::new(config, timeout).await?;

    // PhylumApi may have had to log in, updating the auth info so we should save
    // the config
    let api_config = api.config();
    api_config
        .save()
        .with_context(|| format!("Failed to save configuration to {:?}", api_config.path))?;

    Ok(api)
}

/// Check for an updated release of the CLI
async fn check_for_updates(config: &mut Config) -> Result<()> {
    let now = UNIX_EPOCH.elapsed().expect("Time went backwards").as_secs() as usize;

    if let Some(last_update) = config.last_update {
        const SECS_IN_DAY: usize = 24 * 60 * 60;
        if now.saturating_sub(last_update) <= SECS_IN_DAY {
            log::debug!("Skipping update check...");
            return Ok(());
        }
    }

    log::debug!("Checking for updates...");

    // Update last update check timestamp.
    config.last_update = Some(now);
    config.save().unwrap_or_else(|e| log::error!("Failed to save config: {}", e));

    if update::needs_update(false).await {
        print::print_update_message();
    }

    Ok(())
}

async fn handle_commands() -> CommandResult {
    // Initialize clap app and read configuration.
    //

    let app = phylum_cli::app::app().arg_required_else_help(true).subcommand_required(true);
    let app_name = app.get_name().to_string();
    // Required for printing help messages since `get_matches()` consumes `App`
    let app_helper = &mut app.clone();
    let ver = app.get_version().unwrap().to_owned();
    let matches = app.get_matches();

    // Set the log level based on CLI arguments.
    let log_level = match (matches.get_count("verbose"), matches.get_count("quiet")) {
        (_, 2..) => Some(LevelFilter::Off),
        (_, 1) => Some(LevelFilter::Error),
        (1, _) => Some(LevelFilter::Info),
        (2, _) => Some(LevelFilter::Debug),
        (3.., _) => Some(LevelFilter::Trace),
        // Use environment setup by default.
        (0, 0) => None,
    };

    // Initialize the logger at the desired level.
    match log_level {
        Some(level) => env_logger::Builder::new().filter_level(level).init(),
        None => env_logger::Builder::from_env(Env::default().default_filter_or("warn")).init(),
    };

    let mut config = config::load_config(&matches)?;

    config.set_ignore_certs_cli(matches.get_flag("no-check-certificate"));
    if config.ignore_certs() {
        log::warn!("Ignoring TLS server certificate verification per user request.");
    }

    // We initialize these value here, for later use by the PhylumApi object.
    let timeout = matches.get_one::<String>("timeout").and_then(|t| t.parse::<u64>().ok());

    let (subcommand, sub_matches) = matches.subcommand().unwrap();

    // Check for updates unless we're running without config or the `update`
    // subcommand.
    if cfg!(feature = "selfmanage") && config.path.is_some() && subcommand != "update" {
        check_for_updates(&mut config).await?;
    }

    // Get the future, but don't await. Commands that require access to the API will
    // await on this, so that the API is not instantiated ahead of time for
    // subcommands that don't require it.
    let api = api_factory(config.clone(), timeout);

    match subcommand {
        "auth" => {
            drop(api);
            auth::handle_auth(config, sub_matches, timeout).await
        },
        "version" => handle_version(&app_name, &ver),
        "parse" => parse::handle_parse(sub_matches),
        "parse-sandboxed" => parse::handle_parse_sandboxed(sub_matches),
        "ping" => handle_ping(Spinner::wrap(api).await?).await,
        "project" => {
            project::handle_project(&Spinner::wrap(api).await?, app_helper, sub_matches).await
        },
        "package" => packages::handle_get_package(&Spinner::wrap(api).await?, sub_matches).await,
        "history" => jobs::handle_history(&Spinner::wrap(api).await?, sub_matches).await,
        "group" => group::handle_group(&Spinner::wrap(api).await?, sub_matches).await,
        "analyze" | "batch" => jobs::handle_submission(&Spinner::wrap(api).await?, &matches).await,
        "init" => init::handle_init(&Spinner::wrap(api).await?, sub_matches).await,
        "status" => status::handle_status(sub_matches).await,

        #[cfg(feature = "selfmanage")]
        "uninstall" => uninstall::handle_uninstall(sub_matches),
        #[cfg(feature = "selfmanage")]
        "update" => handle_update(sub_matches, config.ignore_certs()).await,

        "extension" => extensions::handle_extensions(Box::pin(api), sub_matches, app_helper).await,
        #[cfg(unix)]
        "sandbox" => sandbox::handle_sandbox(sub_matches).await,
        "find-dependency-files" => find_dependency_files::handle_command(),
        extension_subcmd => {
            extensions::handle_run_extension(Box::pin(api), extension_subcmd, sub_matches).await
        },
    }
}

async fn handle_ping(api: PhylumApi) -> CommandResult {
    print_user_success!("{}", api.ping().await?);
    Ok(ExitCode::Ok)
}

#[cfg(feature = "selfmanage")]
async fn handle_update(matches: &ArgMatches, ignore_certs: bool) -> CommandResult {
    let res = update::do_update(matches.get_flag("prerelease"), ignore_certs).await;
    let message = res?;
    print_user_success!("{}", message);
    Ok(ExitCode::Ok)
}

fn handle_version(app_name: &str, ver: &str) -> CommandResult {
    print_user_success!("{app_name} (Version {ver}){LICENSE_BLURB}");
    Ok(ExitCode::Ok)
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    match handle_commands().await {
        Ok(code) => code.exit(),
        Err(error) => exit_fail(format!("{error:?}"), ExitCode::Generic),
    }
}
