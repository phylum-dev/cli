use ansi_term::Color::{Blue, Cyan, Green, White};
use chrono::Local;
use clap::{load_yaml, App, AppSettings, ArgMatches};
use dialoguer::{theme::ColorfulTheme, Input, Password, Select};
use home::home_dir;
use serde::Serialize;
use spinners::{Spinner, Spinners};
use std::error::Error;
use std::io;
use std::io::Write;
use std::path::Path;
use std::process;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

extern crate serde;
extern crate serde_json;

use phylum_cli::api::PhylumApi;
use phylum_cli::config::*;
use phylum_cli::lockfiles::Parseable;
use phylum_cli::lockfiles::*;
use phylum_cli::summarize::Summarize;
use phylum_cli::types::*;
use phylum_cli::update::ApplicationUpdater;

const STATUS_THRESHOLD_BREACHED: i32 = 1;

macro_rules! print_user_success {
    ($($tts:tt)*) => {
        eprint!("✅ ",);
        eprintln!($($tts)*);
    }
}

macro_rules! print_user_warning {
    ($($tts:tt)*) => {
        eprint!("⚠️  ",);
        eprintln!($($tts)*);
    }
}

macro_rules! print_user_failure {
    ($($tts:tt)*) => {
        eprint!("❗ ");
        eprintln!($($tts)*);
    }
}

fn print_response<T>(resp: &Result<T, phylum_cli::Error>, pretty_print: bool)
where
    T: std::fmt::Debug + Serialize + Summarize,
{
    log::debug!("==> {:?}", resp);

    match resp {
        Ok(resp) => {
            if pretty_print {
                resp.summarize();
            } else {
                // Use write! as a workaround to avoid https://github.com/rust-lang/rust/issues/46016
                //  when piping output to an external program
                let mut stdout = io::stdout();
                write!(
                    &mut stdout,
                    "{}",
                    serde_json::to_string_pretty(&resp).unwrap_or_else(|e| {
                        log::error!("Failed to serialize json response: {}", e);
                        "".to_string()
                    })
                )
                .unwrap_or_else(|e| log::debug!("Failed writing to stdout: {}", e));
            }
        }
        Err(err) => {
            print_user_failure!("Response error:\n{}", err);
        }
    }
}

fn parse_package(options: &ArgMatches, request_type: &PackageType) -> Option<PackageDescriptor> {
    if !(options.is_present("name") && options.is_present("version")) {
        return None;
    }

    let name = options.value_of("name").unwrap().to_string(); // required option
    let version = options.value_of("version").unwrap().to_string();
    let mut r#type = request_type.to_owned();

    // If a package type was provided on the command line, prefer that
    //  to the global setting
    if options.is_present("type") {
        r#type = PackageType::from_str(options.value_of("type").unwrap()).unwrap_or(r#type);
    }

    Some(PackageDescriptor {
        name,
        version,
        r#type,
    })
}

/// List the projects in this account.
fn get_project_list(api: &mut PhylumApi, pretty_print: bool) {
    let resp = api.get_projects();
    let proj_title = format!("{}", Blue.paint("Project Name"));
    let id_title = format!("{}", Blue.paint("Project ID"));
    println!("{:<38}{}", proj_title, id_title);
    print_response(&resp, pretty_print);
    println!();
}

fn handle_status<T>(
    resp: Result<RequestStatusResponse<T>, phylum_cli::Error>,
    pretty: bool,
) -> Action
where
    T: std::fmt::Debug + Serialize + Summarize,
    phylum_cli::types::RequestStatusResponse<T>: Summarize,
{
    let mut action = Action::None;

    if let Err(phylum_cli::Error::HttpError(404, _)) = resp {
        print_user_warning!(
            "No results found. Submit a lockfile for processing:\n\n\t{}\n",
            Blue.paint("phylum analyze <lock_file>")
        );
    } else {
        if let Ok(ref resp) = resp {
            if !resp.pass {
                action = resp.action.to_owned();
            }
        }
        print_response(&resp, pretty);
    }

    action
}

/// Display user-friendly overview of a job
fn get_job_status(api: &mut PhylumApi, job_id: &JobId, verbose: bool, pretty: bool) -> Action {
    if verbose {
        let resp = api.get_job_status_ext(job_id);
        handle_status(resp, pretty)
    } else {
        let resp = api.get_job_status(job_id);
        handle_status(resp, pretty)
    }
}

/// Handle the history subcommand.
///
/// This allows us to list last N job runs, list the projects, list runs
/// associated with projects, and get the detailed run results for a specific
/// job run.
fn handle_history(api: &mut PhylumApi, config: Config, matches: &clap::ArgMatches) -> Action {
    let pretty_print = !matches.is_present("json");
    let verbose = matches.is_present("verbose");
    let mut ret = Action::None;

    let mut get_job = |job_id: Option<&str>| {
        let job_id_str = job_id.unwrap();

        let job_id = if job_id_str == "current" {
            get_current_project().map(|p: ProjectConfig| p.id)
        } else {
            JobId::from_str(job_id_str).ok()
        }
        .unwrap_or_else(|| exit(Some(&format!("Invalid job id: {}", job_id_str)), -3));

        get_job_status(api, &job_id, verbose, pretty_print)
    };

    if let Some(matches) = matches.subcommand_matches("project") {
        let project_name = matches.value_of("project_name");
        let project_job_id = matches.value_of("job_id");

        if let Some(project_name) = project_name {
            if project_job_id.is_none() {
                let resp = api.get_project_details(project_name);
                print_response(&resp, pretty_print);
            } else {
                ret = get_job(project_job_id);
            }
        } else {
            get_project_list(api, pretty_print);
        }
    } else if matches.is_present("JOB_ID") {
        ret = get_job(matches.value_of("JOB_ID"));
    } else {
        let resp = api.get_status();
        if let Err(phylum_cli::Error::HttpError(404, _)) = resp {
            print_user_warning!(
                "No results found. Submit a lockfile for processing:\n\n\t{}\n",
                Blue.paint("phylum analyze <lock_file>")
            );
        } else {
            println!(
                "Projects and most recent run for {}\n",
                Blue.paint(&config.auth_info.user)
            );
            print_response(&resp, pretty_print);
        }
    }

    ret
}

/// Attempt to get packages from an unknown lockfile type
fn try_get_packages(path: &Path) -> Option<(Vec<PackageDescriptor>, PackageType)> {
    log::warn!(
        "Attempting to obtain packages from unrecognized lockfile type: {}",
        path.to_string_lossy()
    );

    let packages = YarnLock::new(path).ok()?.parse();
    if packages.is_ok() {
        log::debug!("Submitting file as type yarn lock");
        return packages.ok().map(|pkgs| (pkgs, PackageType::Npm));
    }

    let packages = PackageLock::new(path).ok()?.parse();
    if packages.is_ok() {
        log::debug!("Submitting file as type package lock");
        return packages.ok().map(|pkgs| (pkgs, PackageType::Npm));
    }

    let packages = GemLock::new(path).ok()?.parse();
    if packages.is_ok() {
        log::debug!("Submitting file as type gem lock");
        return packages.ok().map(|pkgs| (pkgs, PackageType::Ruby));
    }

    let packages = PyRequirements::new(path).ok()?.parse();
    if packages.is_ok() {
        log::debug!("Submitting file as type pip requirements.txt");
        return packages.ok().map(|pkgs| (pkgs, PackageType::Python));
    }

    let packages = PipFile::new(path).ok()?.parse();
    if packages.is_ok() {
        log::debug!("Submitting file as type pip Pipfile or Pipfile.lock");
        return packages.ok().map(|pkgs| (pkgs, PackageType::Python));
    }

    log::error!("Failed to identify lock file type");
    None
}

/// Determine the lockfile type based on its name and parse
/// accordingly to obtain the packages from it
fn get_packages_from_lockfile(path: &str) -> Option<(Vec<PackageDescriptor>, PackageType)> {
    let path = Path::new(path);
    let file = path.file_name()?.to_str()?;

    let res = match file {
        "Gemfile.lock" => {
            let parser = GemLock::new(path).ok()?;
            parser.parse().ok().map(|pkgs| (pkgs, PackageType::Ruby))
        }
        "package-lock.json" => {
            let parser = PackageLock::new(path).ok()?;
            parser.parse().ok().map(|pkgs| (pkgs, PackageType::Npm))
        }
        "yarn.lock" => {
            let parser = YarnLock::new(path).ok()?;
            parser.parse().ok().map(|pkgs| (pkgs, PackageType::Npm))
        }
        "requirements.txt" => {
            let parser = PyRequirements::new(path).ok()?;
            parser.parse().ok().map(|pkgs| (pkgs, PackageType::Python))
        }
        "Pipfile.txt" | "Pipfile.lock" => {
            let parser = PipFile::new(path).ok()?;
            parser.parse().ok().map(|pkgs| (pkgs, PackageType::Python))
        }
        _ => try_get_packages(path),
    };

    let pkg_count = res.as_ref().map(|p| p.0.len()).unwrap_or_default();

    log::debug!("Read {} packages from file `{}`", pkg_count, file);

    res
}

/// Handles submission of packages to the system for analysis and
/// displays summary information about the submitted package(s)
fn handle_submission(api: &mut PhylumApi, config: Config, matches: &clap::ArgMatches) -> Action {
    let mut packages = vec![];
    let mut request_type = config.request_type; // default request type
    let mut synch = false; // get status after submission
    let mut verbose = false;
    let mut pretty_print = false;
    let mut label = None;
    let mut is_user = true; // is a user (non-batch) request
    let mut ret = Action::None;

    let project = get_current_project()
        .map(|p: ProjectConfig| p.id)
        .unwrap_or_else(|| {
            exit(
                Some("Failed to find a valid project configuration. Did you run `phylum projects create <project-name>`?"),
                -1
            );
        });

    if let Some(matches) = matches.subcommand_matches("analyze") {
        // Should never get here if `LOCKFILE` was not specified
        let lockfile = matches.value_of("LOCKFILE").unwrap();
        let res = get_packages_from_lockfile(lockfile).unwrap_or_else(|| {
            exit(
                Some("Unable to locate any valid package in package lockfile"),
                -1,
            )
        });

        packages = res.0;
        request_type = res.1;

        label = matches.value_of("label");
        verbose = matches.is_present("verbose");
        pretty_print = !matches.is_present("json");
        is_user = !matches.is_present("force");
        synch = true;
    } else if let Some(matches) = matches.subcommand_matches("batch") {
        let mut eof = false;
        let mut line = String::new();
        let mut reader: Box<dyn io::BufRead> = if let Some(file) = matches.value_of("file") {
            // read entries from the file
            Box::new(io::BufReader::new(std::fs::File::open(file).unwrap()))
        } else {
            // read from stdin
            log::info!("Waiting on stdin...");
            Box::new(io::BufReader::new(io::stdin()))
        };

        // If a package type was provided on the command line, prefer that
        //  to the global setting
        if matches.is_present("type") {
            request_type =
                PackageType::from_str(matches.value_of("type").unwrap()).unwrap_or(request_type);
        }
        label = matches.value_of("label");
        is_user = !matches.is_present("force");

        while !eof {
            match reader.read_line(&mut line) {
                Ok(0) => eof = true,
                Ok(_) => {
                    line.pop();
                    let pkg_info = line.split(':').collect::<Vec<&str>>();
                    if pkg_info.len() != 2 {
                        log::debug!("Invalid package input: `{}`", line);
                        continue;
                    }
                    packages.push(PackageDescriptor {
                        name: pkg_info[0].to_owned(),
                        version: pkg_info[1].to_owned(),
                        r#type: request_type.to_owned(),
                    });
                    line.clear();
                }
                Err(err) => {
                    err_exit(err, "Error reading input", -6);
                }
            }
        }
    }

    log::debug!("Submitting request...");
    let job_id = api
        .submit_request(
            &request_type,
            &packages,
            is_user,
            project,
            label.map(|s| s.to_string()),
        )
        .unwrap_or_else(|err| err_exit(err, "Error submitting package", -2));

    log::debug!("Response => {:?}", job_id);
    print_user_success!("Job ID: {}", job_id);

    if synch {
        log::debug!("Requesting status...");
        ret = get_job_status(api, &job_id, verbose, pretty_print);
    }

    ret
}

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
            err_exit(err, "Error registering user", -1);
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
        err_exit(err, "", -1);
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
            .unwrap_or_else(|err| err_exit(err, "Received invalid token id", -5));
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
        print_response(&res, true);
        println!();
    }
}

/// Display the current authentication status to the user.
fn handle_auth_status(api: &mut PhylumApi, config: &mut Config) {
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
fn handle_auth(
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

/// Handle the project subcommand. Provides facilities for creating a new project,
/// linking a current repository to an existing project, listing projects and
/// setting project thresholds for risk domains.
fn handle_projects(api: &mut PhylumApi, matches: &clap::ArgMatches) -> i32 {
    let pretty_print = !matches.is_present("json");

    if let Some(matches) = matches.subcommand_matches("create") {
        let project_name = matches.value_of("name").unwrap();

        log::info!("Initializing new project: `{}`", project_name);
        let project_id = api.create_project(project_name).unwrap_or_else(|err| {
            err_exit(err, "Error initializing project", -1);
        });

        let proj_conf = ProjectConfig {
            id: project_id.to_owned(),
            name: project_name.to_owned(),
            created_at: Local::now(),
        };

        save_config(PROJ_CONF_FILE, &proj_conf).unwrap_or_else(|err| {
            print_user_failure!("Failed to save user projects file: {}", err);
        });

        print_user_success!("Successfully created new project, {}", project_name);
        return 0;
    } else if matches.subcommand_matches("list").is_some() {
        get_project_list(api, pretty_print);
    } else if let Some(matches) = matches.subcommand_matches("link") {
        let project_name = matches.value_of("name").unwrap();
        let resp = api.get_project_details(project_name);

        match resp {
            Ok(proj) => {
                let proj_uuid = Uuid::parse_str(proj.id.as_str()).unwrap(); // TODO: Handle this.
                let proj_conf = ProjectConfig {
                    id: proj_uuid,
                    name: proj.name,
                    created_at: Local::now(),
                };
                save_config(PROJ_CONF_FILE, &proj_conf).unwrap_or_else(|err| {
                    log::error!("Failed to save user credentials to config: {}", err)
                });

                print_user_success!(
                    "Linked the current working directory to the project {}.",
                    format!("{}", White.paint(proj_conf.name))
                );
            }
            Err(x) => {
                print_user_failure!("A project with that name does not exist: {}", x);
                return -1;
            }
        }
    } else if let Some(matches) = matches.subcommand_matches("set-thresholds") {
        let mut project_name = matches.value_of("name").unwrap_or("current");

        let proj = if project_name == "current" {
            get_current_project().map(|p| p.name)
        } else {
            None
        };

        project_name = proj.as_deref().unwrap_or(project_name);
        log::debug!("Setting thresholds for project `{}`", project_name);

        println!("Risk thresholds allow you to specify what constitutes a failure.");
        println!("You can set a threshold for the overall project score, or for individual");
        println!("risk vectors:");
        println!();
        println!("    * Author");
        println!("    * Malicious Code");
        println!("    * Vulnerability");
        println!("    * License");
        println!("    * Engineering");
        println!();
        println!("If your project score falls below a given threshold, it will be");
        println!("considered a failure and the action you specify will be taken.");
        println!();
        println!("Possible actions are:");
        println!();
        println!(
            "    * {}: print a message to standard error",
            format!("{}", White.paint("Print a warning"))
        );
        println!(
            "    * {}: If we are in CI/CD break the build and return a non-zero exit code",
            format!("{}", White.paint("Break the build"))
        );
        println!(
            "    * {}: Ignore the failure and continue",
            format!("{}", White.paint("Nothing, fail silently"))
        );
        println!();

        println!("Specify the thresholds and actions for {}. A threshold of zero will disable the threshold.", format!("{}", White.paint(project_name)));
        println!();

        let project_details = match api.get_project_details(project_name) {
            Ok(x) => x,
            Err(e) => {
                log::error!("Failed to get projet details: {}", e);
                print_user_failure!("Could not get project details");
                return -1;
            }
        };

        let mut user_settings = match api.get_user_settings() {
            Ok(x) => x,
            Err(e) => {
                log::error!("Failed to get user settings: {}", e);
                print_user_failure!("Could not get user settings");
                return -1;
            }
        };

        for threshold_name in vec![
            "total project",
            "author",
            "engineering",
            "license",
            "malicious code",
            "vulnerability",
        ]
        .iter()
        {
            let (threshold, action) = prompt_threshold(threshold_name).unwrap_or((0, "none"));

            // API expects slight key change for specific fields.
            let name = match *threshold_name {
                "total project" => String::from("total"),
                "malicious code" => String::from("maliciousCode"),
                x => x.to_string(),
            };

            user_settings.set_threshold(
                project_details.id.clone(),
                name,
                threshold,
                action.to_string(),
            );
        }

        let resp = api.put_user_settings(&user_settings);
        match resp {
            Ok(_) => {
                print_user_success!(
                    "Set all thresholds for the {} project",
                    White.paint(project_name)
                );
            }
            _ => {
                print_user_failure!(
                    "Failed to set thresholds for the {} project",
                    White.paint(project_name)
                );
            }
        }
    } else {
        get_project_list(api, pretty_print);
    }

    0
}

/// Prompt the user for the threshold value and action associated with a given
/// threshold.
fn prompt_threshold(name: &str) -> Result<(i32, &str), std::io::Error> {
    let threshold = Input::with_theme(&ColorfulTheme::default())
        .with_prompt(format!(
            "{} Threshold",
            format!("{}", White.paint(name.to_uppercase()))
        ))
        .validate_with(|input: &String| -> Result<(), &str> {
            if input.chars().all(char::is_numeric) {
                let val = input.parse::<i32>().unwrap();
                if (0..=100).contains(&val) {
                    Ok(())
                } else {
                    Err("Make sure to specify a number between 0-100")
                }
            } else {
                Err("Threshold must be a number between 0-100")
            }
        })
        .interact_text()?;

    if threshold == "0" {
        println!(
            "\nDisabling {} risk domain",
            format!("{}", White.paint(name))
        );
        println!("\n-----\n");
        return Ok((0, "none"));
    }

    println!(
        "\nWhat should happen if a score falls below the {} threshold?\n",
        format!("{}", White.paint(name))
    );

    let items = vec![
        "Break the CI/CD build",
        "Print a warning message",
        "Do nothing",
    ];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .items(&items)
        .default(0)
        .interact()
        .unwrap();
    let action = items[selection];
    println!("✔ {} Action · {}", White.paint(name.to_uppercase()), action);
    println!("\n-----\n");

    Ok((
        threshold.parse::<i32>().unwrap(),
        match selection {
            // Convert the provided selection index into a string suitable for sending
            // back to the API endpoint responsible for handling user settings.
            0 => "break",
            1 => "warn",
            2 => "none",
            _ => "warn", // We shouldn't be able to make it here.
        },
    ))
}

/// Prints a verbose message informing the user that an update is available.
fn print_update_message() {
    eprintln!(
        "---------------- {} ----------------\n",
        Cyan.paint("Update Available")
    );
    eprintln!("A new version of the Phylum CLI is available. Run");
    eprintln!(
        "\n\t{}\n\nto update to the latest version!\n",
        Blue.paint("phylum update")
    );
    eprintln!("{:-^50}\n\n", "");
}

fn print_sc_help(app: &mut App, subcommand: &str) {
    for sc in app.get_subcommands_mut() {
        if sc.get_name() == subcommand {
            let _ = sc.print_help();
            break;
        }
    }
    println!();
}

/// Handle the subcommands for the `package` subcommand.
fn handle_get_package(
    api: &mut PhylumApi,
    req_type: &PackageType,
    matches: &clap::ArgMatches,
) -> i32 {
    let pretty_print = !matches.is_present("json");
    let pkg = parse_package(matches, req_type);
    if pkg.is_none() {
        return -1;
    }
    let resp = api.get_package_details(&pkg.unwrap());
    log::debug!("==> {:?}", resp);

    if let Err(phylum_cli::Error::HttpError(404, _)) = resp {
        print_user_warning!(
            "No matching packages found. Submit a lockfile for processing:\n\n\t{}\n",
            Blue.paint("phylum analyze <lock_file>")
        );
    } else {
        print_response(&resp, pretty_print);
    }

    0
}

fn authenticate(
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

    if api.api_key.is_none() {
        log::debug!("using standard auth");
        let resp = api
            .authenticate(&config.auth_info.user, &config.auth_info.pass)
            .map(|_t| ());
        log::debug!("==> {:?}", resp);
        return resp;
    }

    Ok(())
}

fn main() {
    env_logger::init();

    let yml = load_yaml!(".conf/cli.yaml");
    let app = App::from(yml)
        .setting(AppSettings::ArgRequiredElseHelp)
        .setting(AppSettings::SubcommandRequiredElseHelp);
    let ver = &app.render_version();

    // Required for printing help messages since `get_matches()` consumes `App`
    let app_helper = &mut app.clone();

    let matches = app.get_matches();
    let mut exit_status: i32 = 0;
    let mut action = Action::None;

    let home_path = home_dir().unwrap_or_else(|| {
        exit(Some("Couldn't find the user's home directory"), -1);
    });
    let settings_path = home_path.as_path().join(".phylum").join("settings.yaml");

    let config_path = matches.value_of("config").unwrap_or_else(|| {
        settings_path.to_str().unwrap_or_else(|| {
            log::error!("Unicode parsing error in configuration file path");
            exit(
                Some(&format!(
                    "Unable to read path to configuration file at '{:?}'",
                    settings_path
                )),
                -1,
            );
        })
    });
    log::debug!("Reading config from {}", config_path);

    let mut config: Config = read_configuration(config_path).unwrap_or_else(|err| {
        exit(
            Some(&format!(
                "Failed to read configuration at `{}`: {}",
                config_path, err
            )),
            -1,
        );
    });

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
        match updater.get_latest_version() {
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
            exit(None, 0);
        }
    } else if let Some(matches) = matches.subcommand_matches("package") {
        if !(matches.is_present("name") && matches.is_present("version")) {
            print_sc_help(app_helper, "package");
            exit(None, 0);
        }
    }

    if matches.subcommand_matches("version").is_some() {
        let name = yml["name"].as_str().unwrap_or("");
        let version = yml["version"].as_str().unwrap_or("");
        print_user_success!("{} (Version {})", name, version);
        exit(None, 0);
    }

    let timeout = matches
        .value_of("timeout")
        .and_then(|t| t.parse::<u64>().ok());
    let mut api = PhylumApi::new(&config.connection.uri, timeout).unwrap_or_else(|err| {
        err_exit(err, "Error creating client", -1);
    });

    if matches.subcommand_matches("ping").is_some() {
        let resp = api.ping();
        print_response(&resp, true);
        exit(None, 0);
    }

    let should_projects = matches.subcommand_matches("projects").is_some();
    let should_submit = matches.subcommand_matches("analyze").is_some()
        || matches.subcommand_matches("batch").is_some();
    let should_get_history = matches.subcommand_matches("history").is_some();
    let should_cancel = matches.subcommand_matches("cancel").is_some();
    let should_get_packages = matches.subcommand_matches("package").is_some();

    let auth_subcommand = matches.subcommand_matches("auth");
    let should_manage_tokens = auth_subcommand.is_some()
        && auth_subcommand
            .unwrap()
            .subcommand_matches("keys")
            .is_some();

    if should_projects
        || should_submit
        || should_get_history
        || should_cancel
        || should_manage_tokens
        || should_get_packages
    {
        let res = authenticate(&mut api, &mut config, should_manage_tokens);

        if let Err(e) = res {
            err_exit(e, "Error attempting to authenticate", -1);
        }
    }

    if let Some(matches) = matches.subcommand_matches("projects") {
        exit_status = handle_projects(&mut api, matches);
    } else if let Some(matches) = matches.subcommand_matches("auth") {
        handle_auth(&mut api, &mut config, config_path, matches, app_helper);
    } else if matches.subcommand_matches("update").is_some() {
        let sp = Spinner::new(
            Spinners::Dots12,
            "Downloading update and verifying binary signatures...".into(),
        );
        let updater = ApplicationUpdater::default();
        match updater.get_latest_version() {
            Some(ver) => match updater.do_update(ver) {
                Ok(msg) => {
                    sp.stop();
                    println!();
                    print_user_success!("{}", msg);
                }
                Err(msg) => {
                    sp.stop();
                    println!();
                    print_user_failure!("{}", msg);
                }
            },
            _ => {
                sp.stop();
                println!();
                print_user_warning!("Failed to get version metadata");
            }
        };
    } else if let Some(matches) = matches.subcommand_matches("package") {
        exit_status = handle_get_package(&mut api, &config.request_type, matches);
    } else if should_submit {
        action = handle_submission(&mut api, config, &matches);
    } else if let Some(matches) = matches.subcommand_matches("history") {
        action = handle_history(&mut api, config, matches);
    } else if should_cancel {
        if let Some(matches) = matches.subcommand_matches("cancel") {
            let request_id = matches.value_of("request_id").unwrap().to_string();
            let request_id = JobId::from_str(&request_id)
                .unwrap_or_else(|err| err_exit(err, "Received invalid request id. Request id's should be of the form xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx", -4));
            let resp = api.cancel(&request_id);
            print_response(&resp, true);
        }
    }

    match action {
        Action::None => {
            log::debug!("Exiting with status {}", exit_status);
            exit(None, exit_status)
        }
        Action::Warn => exit(Some("Project failed threshold requirements!"), exit_status),
        Action::Break => exit(
            Some("Project failed threshold requirements, failing the build!"),
            STATUS_THRESHOLD_BREACHED,
        ),
    }
}

fn err_exit(error: impl Error, message: &str, code: i32) -> ! {
    log::error!("{}: {:?}", message, error);
    print_user_failure!("Error: {}", message);
    process::exit(code);
}

fn exit(message: Option<&str>, code: i32) -> ! {
    if let Some(message) = message {
        if code != 0 {
            log::warn!("{}", message);
            print_user_failure!("Error: {}", message);
        } else {
            log::debug!("{}", message);
            print_user_warning!("{}", message);
        }
    }
    process::exit(code);
}
