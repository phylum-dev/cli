use ansi_term::Color::{Blue, Cyan, Green, White};
use chrono::Local;
use clap::{load_yaml, App, AppSettings, ArgMatches};
use dialoguer::{theme::ColorfulTheme, Input, Password, Select};
use home::home_dir;
use serde::Serialize;
use spinners::{Spinner, Spinners};
use std::error::Error;
use std::fs;
use std::io::ErrorKind;
use std::io::Write;
use std::io::{self, BufRead, BufReader};
use std::os::unix::fs::PermissionsExt;
use std::process;
use std::str::FromStr;
use uuid::Uuid;

extern crate serde;
extern crate serde_json;

use phylum_cli::api::PhylumApi;
use phylum_cli::config::*;
use phylum_cli::render::Renderable;
use phylum_cli::types::*;

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

fn print_response<T>(resp: &Result<T, phylum_cli::Error>, pretty: bool)
where
    T: Serialize + Renderable,
{
    match resp {
        Ok(resp) => {
            if pretty {
                println!("{}", resp.render())
            } else {
                println!("{}", serde_json::to_string_pretty(&resp).unwrap());
            }
        }
        Err(err) => {
            print_user_failure!("Response error:\n{}", err);
        }
    }
}

fn parse_package(options: &ArgMatches, request_type: &PackageType) -> PackageDescriptor {
    let name = options.value_of("name").unwrap().to_string(); // required option
    let version = options.value_of("version").unwrap_or_default().to_string();
    let mut r#type = request_type.to_owned();

    // If a package type was provided on the command line, prefer that
    //  to the global setting
    if options.is_present("type") {
        r#type = PackageType::from_str(options.value_of("type").unwrap()).unwrap_or(r#type);
    }

    PackageDescriptor {
        name,
        version,
        r#type,
    }
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

/// Handle the history subcommand.
///
/// This allows us to list last N job runs, list the projects, list runs
/// associated with projects, and get the detailed run results for a specific
/// job run.
fn handle_history(api: &mut PhylumApi, config: Config, matches: &clap::ArgMatches) -> i32 {
    let pretty_print = !matches.is_present("json");

    if let Some(matches) = matches.subcommand_matches("project") {
        let project_name = matches.value_of("project_name");
        let project_job_id = matches.value_of("job_id");

        if project_job_id.is_some() && project_name.is_none() {
            println!("TODO: Need functionality from `analyze`");
        } else if project_name.is_some() {
            if project_job_id.is_none() {
                let resp = api.get_project_details(project_name.unwrap());
                print_response(&resp, pretty_print);
            } else {
                println!("TODO: Need functionality from `analyze`");
            }
        } else {
            get_project_list(api, pretty_print);
        }
    } else {
        println!(
            "Projects and most recent run for {}\n",
            Blue.paint(&config.auth_info.user)
        );
        let resp = api.get_status();
        print_response(&resp, pretty_print);
    }

    0
}

fn handle_status(api: &mut PhylumApi, req_type: &PackageType, matches: clap::ArgMatches) -> i32 {
    let mut exit_status: i32 = 0;

    if let Some(matches) = matches.subcommand_matches("status") {
        let pretty_print = !matches.is_present("json");
        let mut threshold: f64 = 0.0;
        if let Some(thresh) = matches.value_of("threshold") {
            threshold = thresh.parse::<f64>().unwrap_or_default();
        };
        if let Some(request_id) = matches.value_of("request_id") {
            let request_id = JobId::from_str(&request_id)
                .unwrap_or_else(|err| exit(err, "Received invalid request id", -3));

            if matches.is_present("verbose") {
                let resp = api.get_job_status_ext(&request_id);
                log::debug!("==> {:?}", resp);
                print_response(&resp, pretty_print);
                if let Ok(resp) = resp {
                    for p in resp.packages {
                        if let Some(score) = p.basic_status.package_score {
                            if score < threshold {
                                exit_status = STATUS_THRESHOLD_BREACHED;
                            }
                        }
                    }
                }
            } else {
                let resp = api.get_job_status(&request_id);
                log::debug!("==> {:?}", resp);
                print_response(&resp, pretty_print);
                if let Ok(resp) = resp {
                    for p in resp.packages {
                        if let Some(score) = p.package_score {
                            if score < threshold {
                                exit_status = STATUS_THRESHOLD_BREACHED;
                            }
                        }
                    }
                }
            }
        } else if matches.is_present("name") {
            if !matches.is_present("version") {
                print_user_failure!("A version is required when querying by package");
                process::exit(-3);
            }
            let pkg = parse_package(matches, &req_type);
            let resp = api.get_package_details(&pkg);
            log::debug!("==> {:?}", resp);
            print_response(&resp, pretty_print);
            if let Ok(resp) = resp {
                if let Some(score) = resp.basic_status.package_score {
                    if score < threshold {
                        exit_status = STATUS_THRESHOLD_BREACHED;
                    }
                }
            }
        } else {
            // get everything
            let resp = api.get_status();
            log::debug!("==> {:?}", resp);
        }
    }

    exit_status
}

fn handle_submission(api: &mut PhylumApi, config: Config, matches: clap::ArgMatches) -> i32 {
    let mut exit_status: i32 = 0;

    // If any packages were listed in the config file, include
    //  those as well.
    let mut packages = config.packages.unwrap_or_default();
    let mut request_type = config.request_type;
    let mut is_user = true;
    let mut synch = true;

    let project = find_project_conf(".")
        .and_then(|s| parse_config(&s).ok())
        .map(|p: ProjectConfig| p.id)
        .unwrap_or_else(|| {
            print_user_failure!(
                "Failed to find a valid project configuration. Did you run `phylum projects create <project-name>`?"
            );
            process::exit(-1)
        });

    let mut label = None;

    if let Some(matches) = matches.subcommand_matches("submit") {
        let pkg = parse_package(matches, &request_type);
        request_type = pkg.r#type.to_owned();
        packages.push(pkg);
        is_user = !matches.is_present("low-priority");
        synch = !matches.is_present("synch");
        label = matches.value_of("label");
    } else if let Some(matches) = matches.subcommand_matches("batch") {
        let mut eof = false;
        let mut line = String::new();
        let mut reader: Box<dyn BufRead> = if let Some(file) = matches.value_of("file") {
            // read entries from the file
            Box::new(BufReader::new(std::fs::File::open(file).unwrap()))
        } else {
            // read from stdin
            log::info!("Waiting on stdin...");
            Box::new(BufReader::new(io::stdin()))
        };

        // If a package type was provided on the command line, prefer that
        //  to the global setting
        if matches.is_present("type") {
            request_type =
                PackageType::from_str(matches.value_of("type").unwrap()).unwrap_or(request_type);
        }
        label = matches.value_of("label");

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
                    exit(err, "Error reading input", -6);
                }
            }
        }
        is_user = !matches.is_present("low-priority");
        synch = !matches.is_present("synch");
    }
    log::debug!("Submitting request...");
    let resp = api
        .submit_request(
            &request_type,
            &packages,
            is_user,
            project,
            label.map(|s| s.to_string()),
        )
        .unwrap_or_else(|err| exit(err, "Error submitting package", -2));
    if synch {
        exit_status = handle_status(api, &request_type, matches);
    }
    log::info!("Response => {:?}", resp);
    print_user_success!("Job ID: {}", resp);
    exit_status
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
            exit(err, "Error registering user", -1);
        });

    config.auth_info.user = email;
    config.auth_info.pass = password;
    save_config(config_path, &config).unwrap_or_else(|err| {
        log::error!("Failed to save user credentials to config: {}", err);
        print_user_failure!("Failed to save user credentials: {}", err);
    });

    Ok("Successfully registred a new account!".to_string())
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
        print_user_failure!("{}", err);
        process::exit(-1);
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
            .unwrap_or_else(|err| exit(err, "Received invalid token id", -5));
        let resp = api.delete_api_token(&token);
        log::info!("==> {:?}", resp);
        config.auth_info.api_token = None;
        save_config(config_path, &config)
            .unwrap_or_else(|err| log::error!("Failed to clear api token from config: {}", err));
        print_user_success!("Successfully deleted API key");
    } else if matches.subcommand_matches("list").is_some() {
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
fn handle_auth_status(config: &mut Config) {
    if config.auth_info.api_token.is_some() {
        let key = config.auth_info.api_token.as_ref().unwrap().key.to_string();
        print_user_success!("Currenty authenticated with API key {}", Green.paint(key));
    } else if !config.auth_info.user.is_empty() {
        print_user_success!(
            "Currenty authenticated as {}",
            Green.paint(&config.auth_info.user)
        );
    }
}

/// Handle the subcommands for the `auth` subcommand.
fn handle_auth(
    api: &mut PhylumApi,
    config: &mut Config,
    config_path: &str,
    matches: &clap::ArgMatches,
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
        handle_auth_status(config);
    } else {
        // TODO: What if we don't have a subcommand? Clap will give us the help
        //       output if the top level subcommands are missing, but not for
        //       sub-subcommands, i.e. `phylum auth` won't produce the help
        //       output.
        print_user_failure!("Missing subcommand.");
    }
}

/// Generic function for fetching data from Github.
fn get_github<T>(url: &str, f: impl Fn(reqwest::blocking::Response) -> Option<T>) -> Option<T> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("phylum-cli")
        .build();

    match client {
        Ok(c) => {
            let resp = c.get(url).send();

            match resp {
                Ok(r) => f(r),
                Err(_) => None,
            }
        }
        Err(_) => None,
    }
}

/// Check for an update by querying the Github releases page.
fn get_latest_version() -> Option<GithubRelease> {
    let url = "https://api.github.com/repos/phylum-dev/cli/releases/latest";
    get_github(url, |r| -> Option<GithubRelease> {
        let data = r.json::<GithubRelease>();

        match data {
            Ok(d) => Some(d),
            Err(e) => {
                println!("Failed latest version check: {:?}", e);
                None
            }
        }
    })
}

/// Download the binary specified in the Github release.
///
/// On success, writes the requested file to the destination `dest`. Returns
/// the number of bytes written.
fn download_file(latest: &GithubReleaseAsset, dest: &str) -> Option<usize> {
    get_github(latest.browser_download_url.as_str(), |r| -> Option<usize> {
        let data = r.bytes().ok()?;

        let mut file = std::fs::File::create(dest).expect("Failed to create temporary update file");
        file.write_all(&data).expect("Failed to write update file");

        Some(data.len())
    })
}

/// Compare the current version as reported by Clap with the version currently
/// published on Github. We do the naive thing here: If the latest version on
/// Github does not match the Clap version, we update.
fn needs_update(latest: &Option<GithubRelease>, current_version: &str) -> bool {
    match latest {
        Some(github) => {
            // The version comes back to us _possibly_ prefixed with `phylum v`.
            // Additionally, Clap returns `phylum <version>` without the "v"`.
            // Normalize the version strings here for comparison.
            let latest = github.name.replace("phylum ", "").replace("v", "");
            let current = current_version.replace("phylum ", "");

            latest != current
        }
        _ => false,
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
        let project_id = api.create_project(&project_name).unwrap_or_else(|err| {
            exit(err, "Error initializing project", -1);
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
        let project_name = matches.value_of("name").unwrap();

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
            _ => {
                print_user_failure!("Could not get project details");
                return -1;
            }
        };

        let mut user_settings = match api.get_user_settings() {
            Ok(x) => x,
            _ => {
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

/// Update the Phylum installation. Please note, this will only function on
/// Linux x64. This is due in part to the fact that the release is only
/// compiling for this OS and architecture.
///
/// Until we update the releases, this should suffice.
fn update_in_place(latest: GithubRelease) -> Result<String, std::io::Error> {
    // We download the update to a temporary location.
    let tmp_bin_path = "/tmp/phylum.update";
    let tmp_bash_path = "/tmp/phylum-bash.update";

    // This is path to the binary on disk.
    let current_bin = std::env::current_exe()?;
    let mut current_bash = current_bin.clone();
    current_bash.pop();
    current_bash.push("phylum-cli.bash");
    let latest_version = &latest.name;

    // The data comes back to us as a JSON response of assets. We do not need
    // every asset. We need the updated binary and the bash file. This simply
    // loops over this data to find the download URLs for each pertinent asset.
    let bin_asset = &latest.assets.iter().find(|x| x.name == "phylum").unwrap();

    let bash_asset = &latest
        .assets
        .iter()
        .find(|x| x.name == "phylum-cli.bash")
        .unwrap();

    // Download the required files for the update.
    let sp = Spinner::new(Spinners::Dots12, "Downloading update...".into());
    let bin = download_file(bin_asset, tmp_bin_path);
    let bash = download_file(bash_asset, tmp_bash_path);
    sp.stop();
    println!();

    // Ensure that we have both files for our update. This includes the actual
    // binary file, as well as the bash file.
    if bin.is_none() || bash.is_none() {
        return Err(std::io::Error::new(
            ErrorKind::Other,
            "Failed to download update files",
        ));
    }

    // If the download succeeds, _then_ we move it to overwrite the
    // existing binary and bash file.
    fs::rename(tmp_bin_path, &current_bin)?;
    fs::rename(tmp_bash_path, &current_bash)?;
    match fs::set_permissions(&current_bin, fs::Permissions::from_mode(0o770)) {
        Ok(_) => {}
        Err(_) => {
            print_user_warning!(
                "Successfully downloaded updates, but failed to make binary executable"
            );
        }
    };

    Ok(format!("Successfully updated to {}!", latest_version))
}

/// Prints a verbose message informing the user that an update is available.
fn print_update_message() {
    println!(
        "---------------- {} ----------------\n",
        Cyan.paint("Update Available")
    );
    println!("A new version of the Phylum CLI is available. Run");
    println!(
        "\n\t{}\n\nto update to the latest version!\n",
        Blue.paint("phylum update")
    );
    println!("{:-^50}\n\n", "");
}

fn main() {
    env_logger::init();

    let yml = load_yaml!(".conf/cli.yaml");
    let app = App::from(yml)
        .setting(AppSettings::ArgRequiredElseHelp)
        .setting(AppSettings::SubcommandRequiredElseHelp);
    let ver = &app.render_version();
    let matches = app.get_matches();
    let mut exit_status: i32 = 0;

    let latest_version = get_latest_version();
    if matches.subcommand_matches("update").is_none() && needs_update(&latest_version, ver) {
        print_update_message();
    }

    // TODO: determine from options
    let pretty_print = false; // json output

    if matches.subcommand_matches("version").is_some() {
        let name = yml["name"].as_str().unwrap_or("");
        let version = yml["version"].as_str().unwrap_or("");
        print_user_success!("{} (Version {})", name, version);
        process::exit(0);
    }
    let home_path = home_dir().unwrap_or_else(|| {
        log::error!("Couldn't find the user's home directory");
        process::exit(-1);
    });
    let settings_path = home_path.as_path().join(".phylum").join("settings.yaml");

    let config_path = matches.value_of("config").unwrap_or_else(|| {
        settings_path.to_str().unwrap_or_else(|| {
            log::error!("Unicode parsing error in configuration file path");
            print_user_failure!(
                "Unable to read path to configuration file at '{:?}'",
                settings_path
            );
            process::exit(-1)
        })
    });
    log::debug!("Reading config from {}", config_path);

    let mut config: Config = read_configuration(config_path).unwrap_or_else(|err| {
        log::error!("Failed to read configuration: {:?}", err);
        print_user_failure!("Failed to read configuration [`{}`]: {}", config_path, err);
        process::exit(-1)
    });

    let timeout = matches
        .value_of("timeout")
        .and_then(|t| t.parse::<u64>().ok());
    let mut api = PhylumApi::new(&config.connection.uri, timeout).unwrap_or_else(|err| {
        exit(err, "Error creating client", -1);
    });

    if matches.subcommand_matches("ping").is_some() {
        let resp = api.ping();
        print_response(&resp, pretty_print);
        process::exit(0);
    }

    let should_projects = matches.subcommand_matches("projects").is_some();
    let should_submit = matches.subcommand_matches("submit").is_some()
        || matches.subcommand_matches("batch").is_some();
    let should_get_history = matches.subcommand_matches("history").is_some();
    let should_cancel = matches.subcommand_matches("cancel").is_some();
    let should_do_heuristics = matches.subcommand_matches("heuristics").is_some();

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
        || should_do_heuristics
    {
        log::debug!("Authenticating...");
        log::debug!("Auth config:\n{:?}", config.auth_info);
        // If an API token has been configured, prefer that.  Otherwise, log in with
        //  a standard username and password to get a JWT.
        if !should_manage_tokens {
            // endpoint doesn't support token auth yet
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
                .unwrap_or_else(|err| {
                    exit(err, "Error attempting to authenticate", -1);
                });

            log::info!("==> {:?}", resp);
        }
    }

    if let Some(matches) = matches.subcommand_matches("projects") {
        exit_status = handle_projects(&mut api, matches);
    } else if let Some(matches) = matches.subcommand_matches("auth") {
        handle_auth(&mut api, &mut config, config_path, matches);
    } else if matches.subcommand_matches("update").is_some() {
        match latest_version {
            Some(ver) => match update_in_place(ver) {
                Ok(msg) => {
                    print_user_success!("{}", msg);
                }
                Err(msg) => {
                    print_user_failure!("{}", msg);
                }
            },
            _ => {
                print_user_warning!("Failed to get version metadata");
            }
        };
    } else if should_submit {
        exit_status = handle_submission(&mut api, config, matches);
    } else if let Some(matches) = matches.subcommand_matches("history") {
        exit_status = handle_history(&mut api, config, matches);
    } else if should_cancel {
        if let Some(matches) = matches.subcommand_matches("cancel") {
            let request_id = matches.value_of("request_id").unwrap().to_string();
            let request_id = JobId::from_str(&request_id)
                .unwrap_or_else(|err| exit(err, "Received invalid request id", -4));
            let resp = api.cancel(&request_id);
            log::info!("==> {:?}", resp);
            print_response(&resp, pretty_print);
        }
    } else if should_do_heuristics {
        let matches = matches.subcommand_matches("heuristics").unwrap();
        if let Some(matches) = matches.subcommand_matches("submit") {
            let pkg = parse_package(matches, &config.request_type);
            let heuristics = matches
                .value_of("heuristics")
                .unwrap_or_default()
                .split(',')
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<String>>();
            let resp = api.submit_heuristics(&pkg, &heuristics, matches.is_present("include-deps"));
            log::info!("==> {:?}", resp);
            print_response(&resp, pretty_print);
        } else {
            log::info!("Querying heuristics");
            let resp = api.query_heuristics();
            log::info!("==> {:?}", resp);
            //print_response(&resp, pretty_print);
        }
    }
    log::debug!("Exiting with status {}", exit_status);
    process::exit(exit_status);
}

fn exit(error: impl Error, message: &str, code: i32) -> ! {
    log::error!("{}: {:?}", message, error);
    print_user_failure!("Error: {}", message);
    process::exit(code);
}
