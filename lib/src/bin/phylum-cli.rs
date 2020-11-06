use ansi_term::Color::{Green, Red};
use clap::{load_yaml, App, AppSettings};
use serde::Serialize;
use std::error::Error;
use std::io::{self, BufRead, BufReader};
use std::process;
use std::str::FromStr;

use phylum_cli::api::PhylumApi;
use phylum_cli::config::{parse_config, save_config};
use phylum_cli::types::{JobId, Key, PackageDescriptor};

macro_rules! print_user_success {
    ($($tts:tt)*) => {
        print!("[{}] ", Green.paint("success"));
        println!($($tts)*);
    }
}

macro_rules! print_user_failure {
    ($($tts:tt)*) => {
        print!("[{}] ", Red.paint("failure"));
        println!($($tts)*);
    }
}

fn print_response<T>(resp: Result<T, phylum_cli::Error>)
where
    T: Serialize,
{
    match resp {
        Ok(resp) => {
            print_user_success!(
                "Response object:\n{}",
                serde_json::to_string_pretty(&resp).unwrap()
            );
        }
        Err(err) => {
            print_user_failure!("Response error:\n{}", err);
        }
    }
}

fn main() {
    env_logger::init();

    let yml = load_yaml!(".conf/cli.yaml");
    let app = App::from(yml).setting(AppSettings::ArgRequiredElseHelp);
    let matches = app.get_matches();

    if matches.subcommand_matches("version").is_some() {
        let name = yml["name"].as_str().unwrap_or("");
        let version = yml["version"].as_str().unwrap_or("");
        print_user_success!("{} (Version {})", name, version);
        process::exit(0);
    }
    let config_path = matches
        .value_of("config")
        .unwrap_or("$HOME/.phylum/settings.yaml");
    log::debug!("Reading config from {}", config_path);

    let mut config = parse_config(config_path).unwrap_or_else(|err| {
        log::error!("Failed to parse config: {:?}", err);
        print_user_failure!(
            "Unable to parse configuration file at `{}`: {}",
            config_path,
            err
        );
        process::exit(-1)
    });

    let mut api = PhylumApi::new(&config.connection.uri).unwrap_or_else(|err| {
        exit(err, "Error creating client", -1);
    });

    let should_submit = matches.subcommand_matches("submit").is_some()
        || matches.subcommand_matches("batch").is_some();
    let should_get_status = matches.subcommand_matches("status").is_some();
    let should_cancel = matches.subcommand_matches("cancel").is_some();
    let should_manage_tokens = matches.subcommand_matches("tokens").is_some();

    if should_submit || should_get_status || should_cancel || should_manage_tokens {
        log::debug!("Authenticating...");
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

    if let Some(matches) = matches.subcommand_matches("register") {
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
        save_config(config_path, &config).unwrap();
        save_config(config_path, &config).unwrap_or_else(|err| {
            log::error!("Failed to save user credentials to config: {}", err)
        });
        print_user_success!("{}", "Successfully registered.");
    } else if should_submit {
        // If any packages were listed in the config file, include
        //  those as well.
        let mut packages = config.packages.unwrap_or_default();
        let request_type = config.request_type;
        let mut is_user = true;
        let mut no_recurse = true;

        if let Some(matches) = matches.subcommand_matches("submit") {
            // If a package was explicitly passed on the command line,
            //  include that.
            // These are required options, so `unwrap` is ok
            let name = matches.value_of("name").unwrap().to_string();
            let version = matches.value_of("version").unwrap().to_string();
            packages.push(PackageDescriptor {
                name,
                version,
                r#type: request_type.to_owned(),
            });
            is_user = !matches.is_present("low-priority");
            no_recurse = !matches.is_present("recurse");
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
            no_recurse = !matches.is_present("recurse");
        }
        log::debug!("Submitting request...");
        let resp = api
            .submit_request(&request_type, &packages, is_user, no_recurse)
            .unwrap_or_else(|err| exit(err, "Error submitting package", -2));
        log::info!("Response => {:?}", resp);
        print_user_success!("Job ID: {}", resp);
    } else if should_get_status {
        if let Some(matches) = matches.subcommand_matches("status") {
            if let Some(request_id) = matches.value_of("request_id") {
                let request_id = JobId::from_str(&request_id)
                    .unwrap_or_else(|err| exit(err, "Received invalid request id", -3));
                let resp = api.get_job_status(&request_id);
                log::info!("==> {:?}", resp);
                print_response(resp);
            } else {
                // get everything
                let resp = api.get_status();
                log::info!("==> {:?}", resp);
                print_response(resp);
            }
        }
    } else if should_cancel {
        if let Some(matches) = matches.subcommand_matches("cancel") {
            let request_id = matches.value_of("request_id").unwrap().to_string();
            let request_id = JobId::from_str(&request_id)
                .unwrap_or_else(|err| exit(err, "Received invalid request id", -4));
            let resp = api.cancel(&request_id);
            log::info!("==> {:?}", resp);
            print_response(resp);
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
                print_response(resp);
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
                print_response(resp);
            } else {
                // get everything
                let resp = api.get_api_tokens();
                log::info!("==> {:?}", resp);
                print_response(resp);
            }
        }
    }
}

fn exit(error: impl Error, message: &str, code: i32) -> ! {
    log::error!("{}: {:?}", message, error);
    print_user_failure!("Error: {}", message);
    process::exit(code);
}
