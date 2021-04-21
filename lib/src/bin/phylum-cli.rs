use ansi_term::Color::{Green, Red};
use chrono::Local;
use clap::{load_yaml, App, AppSettings, ArgMatches};
use home::home_dir;
use serde::Serialize;
use std::error::Error;
use std::io::{self, BufRead, BufReader};
use std::process;
use std::str::FromStr;

use phylum_cli::api::PhylumApi;
use phylum_cli::config::*;
use phylum_cli::types::*;

const STATUS_THRESHOLD_BREACHED: i32 = 1;

macro_rules! print_user_success {
    ($($tts:tt)*) => {
        eprint!("[{}] ", Green.paint("success"));
        eprintln!($($tts)*);
    }
}

macro_rules! print_user_failure {
    ($($tts:tt)*) => {
        eprint!("[{}] ", Red.paint("failure"));
        eprintln!($($tts)*);
    }
}

fn print_response<T>(resp: &Result<T, phylum_cli::Error>)
where
    T: Serialize,
{
    match resp {
        Ok(resp) => {
            print_user_success!("Response object:");
            println!("{}", serde_json::to_string_pretty(&resp).unwrap());
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

fn handle_status(api: &mut PhylumApi, req_type: &PackageType, matches: clap::ArgMatches) -> i32 {
    let mut exit_status: i32 = 0;

    if let Some(matches) = matches.subcommand_matches("status") {
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
                print_response(&resp);
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
                print_response(&resp);
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
            print_response(&resp);
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
            print_response(&resp);
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
                "Failed to find a valid project configuration. Did you run `phylum init`?"
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

fn main() {
    env_logger::init();

    let yml = load_yaml!(".conf/cli.yaml");
    let app = App::from(yml).setting(AppSettings::ArgRequiredElseHelp);
    let matches = app.get_matches();
    let mut exit_status: i32 = 0;

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
        print_response(&resp);
        process::exit(0);
    }

    let should_init = matches.subcommand_matches("init").is_some();
    let should_submit = matches.subcommand_matches("submit").is_some()
        || matches.subcommand_matches("batch").is_some();
    let should_get_status = matches.subcommand_matches("status").is_some();
    let should_cancel = matches.subcommand_matches("cancel").is_some();
    let should_manage_tokens = matches.subcommand_matches("tokens").is_some();
    let should_do_heuristics = matches.subcommand_matches("heuristics").is_some();

    if should_init
        || should_submit
        || should_get_status
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

    if let Some(matches) = matches.subcommand_matches("init") {
        let project = matches.value_of("project").unwrap();
        log::info!("Initializing new project: `{}`", project);
        let project_id = api.create_project(&project).unwrap_or_else(|err| {
            exit(err, "Error initializing project", -1);
        });
        let proj_conf = ProjectConfig {
            id: project_id.to_owned(),
            name: project.to_owned(),
            created_at: Local::now(),
        };
        save_config(PROJ_CONF_FILE, &proj_conf).unwrap_or_else(|err| {
            log::error!("Failed to save user credentials to config: {}", err)
        });
        print_user_success!("Successfully created new project. ID: {}", project_id);
    } else if let Some(matches) = matches.subcommand_matches("register") {
        log::debug!("Registering new user");
        let email = matches.value_of("email").unwrap();
        let pass = matches.value_of("password").unwrap();
        let first = matches.value_of("first").unwrap();
        let last = matches.value_of("last").unwrap();
        let user_id = api
            .register(email, pass, first, last)
            .unwrap_or_else(|err| {
                exit(err, "Error registering user", -1);
            });
        log::debug!("Registered user with id: `{}`", user_id);
        config.auth_info.user = email.to_string();
        config.auth_info.pass = pass.to_string();
        save_config(config_path, &config).unwrap_or_else(|err| {
            log::error!("Failed to save user credentials to config: {}", err)
        });
        print_user_success!("{}", "Successfully registered.");
    } else if should_submit {
        exit_status = handle_submission(&mut api, config, matches);
    } else if should_get_status {
        exit_status = handle_status(&mut api, &config.request_type, matches);
    } else if should_cancel {
        if let Some(matches) = matches.subcommand_matches("cancel") {
            let request_id = matches.value_of("request_id").unwrap().to_string();
            let request_id = JobId::from_str(&request_id)
                .unwrap_or_else(|err| exit(err, "Received invalid request id", -4));
            let resp = api.cancel(&request_id);
            log::info!("==> {:?}", resp);
            print_response(&resp);
        }
    } else if should_manage_tokens {
        if let Some(matches) = matches.subcommand_matches("tokens") {
            let should_create = matches.is_present("create");
            let should_destroy = matches.is_present("delete");
            if should_create && should_destroy {
                log::error!("Incompatible options specified: `create` and `delete`");
                process::exit(-5);
            }
            if should_create {
                let resp = api.create_api_token();
                log::info!("==> Token created: `{:?}`", resp);
                if let Ok(ref resp) = resp {
                    config.auth_info.api_token = Some(resp.to_owned());
                    save_config(config_path, &config).unwrap_or_else(|err| {
                        log::error!("Failed to save api token to config: {}", err)
                    });
                }
                print_response(&resp);
            } else if should_destroy {
                let token_id = matches.value_of("delete").unwrap();
                let token = Key::from_str(token_id)
                    .unwrap_or_else(|err| exit(err, "Received invalid token id", -5));
                let resp = api.delete_api_token(&token);
                log::info!("==> {:?}", resp);
                config.auth_info.api_token = None;
                save_config(config_path, &config).unwrap_or_else(|err| {
                    log::error!("Failed to clear api token from config: {}", err)
                });
                print_response(&resp);
            } else {
                // get everything
                let resp = api.get_api_tokens();
                log::info!("==> {:?}", resp);
                print_response(&resp);
            }
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
            print_response(&resp);
        } else {
            log::info!("Querying heuristics");
            let resp = api.query_heuristics();
            log::info!("==> {:?}", resp);
            print_response(&resp);
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
