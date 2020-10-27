use clap::{load_yaml, App};
use std::error::Error;
use std::process;
use std::str::FromStr;

use cli::api::PhylumApi;
use cli::config::parse_config;
use cli::types::{JobId, Key, PackageDescriptor, PackageType};

fn main() {
    env_logger::init();

    let yml = load_yaml!(".conf/cli.yaml");
    let app = App::from_yaml(yml);
    let matches = app.get_matches();

    let config_file = matches.value_of("config").unwrap_or("settings.yaml");
    log::debug!("Reading config from {}", config_file);

    let config = parse_config(config_file).unwrap_or_else(|err| {
        log::error!("Failed to parse config: {:?}", err);
        process::exit(-1);
    });

    let mut api = PhylumApi::new(&config.connection.uri).unwrap_or_else(|err| {
        exit(err, "Error creating client", -1);
    });

    let should_submit = matches.subcommand_matches("submit").is_some();
    let should_get_status = matches.subcommand_matches("status").is_some();
    let should_cancel = matches.subcommand_matches("cancel").is_some();
    let should_manage_tokens = matches.subcommand_matches("tokens").is_some();

    if should_submit || should_get_status || should_cancel || should_manage_tokens {
        log::debug!("Authenticating...");
        let resp = api
            .authenticate(&config.connection.user, &config.connection.pass)
            .unwrap_or_else(|err| {
                exit(err, "Error attempting to authenticate", -1);
            });

        log::debug!("==> {:?}", resp);
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
        log::info!("Registered user with id: `{}`", user_id);
    } else if should_submit {
        if let Some(matches) = matches.subcommand_matches("submit") {
            // If any packages were listed in the config file, include
            //  those as well.
            let mut packages = config.packages.unwrap_or_default();

            // If a package was explicitly passed on the command line,
            //  include that.
            // These are required options, so `unwrap` is ok
            let name = matches.value_of("name").unwrap().to_string();
            let version = matches.value_of("version").unwrap().to_string();
            let pkg_type_str = matches.value_of("type").unwrap_or("npm");
            let pkg_type = PackageType::from_str(pkg_type_str);
            match pkg_type {
                Ok(pkg_type) => {
                    packages.push(PackageDescriptor {
                        name,
                        version,
                        r#type: pkg_type.to_owned(),
                    });
                    log::info!("Submitting request...");
                    let resp = api
                        .submit_request(&pkg_type, &packages)
                        .unwrap_or_else(|err| exit(err, "Error submitting package", -2));
                    log::info!("Response => {:?}", resp);
                }
                _ => {
                    log::error!("Invalid package type specified");
                    process::exit(-1);
                }
            }
        }
    } else if should_get_status {
        if let Some(matches) = matches.subcommand_matches("status") {
            if let Some(request_id) = matches.value_of("request_id") {
                let request_id = JobId::from_str(&request_id)
                    .unwrap_or_else(|err| exit(err, "Received invalid request id", -3));
                let resp = api.get_job_status(&request_id);
                // TODO: pretty print these
                log::info!("==> {:?}", resp);
            } else {
                // get everything
                let resp = api.get_status();
                log::info!("==> {:?}", resp);
            }
        }
    } else if should_cancel {
        if let Some(matches) = matches.subcommand_matches("cancel") {
            let request_id = matches.value_of("request_id").unwrap().to_string();
            let request_id = JobId::from_str(&request_id)
                .unwrap_or_else(|err| exit(err, "Received invalid request id", -4));
            let resp = api.cancel(&request_id);
            log::info!("==> {:?}", resp);
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
            } else if should_destroy {
                let token_id = matches.value_of("delete").unwrap();
                let token = Key::from_str(token_id)
                    .unwrap_or_else(|err| exit(err, "Received invalid token id", -5));
                let resp = api.delete_api_token(&token);
                log::info!("==> {:?}", resp);
            } else {
                // get everything
                let resp = api.get_api_tokens();
                log::info!("==> {:?}", resp);
            }
        }
    }
}

fn exit(error: impl Error, message: &str, code: i32) -> ! {
    log::error!("{}: {:?}", message, error);
    process::exit(code);
}
