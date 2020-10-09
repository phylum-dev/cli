use std::error::Error;
use std::process;
use std::str::FromStr;
use clap::{App, load_yaml};

use cli::api::PhylumApi;
use cli::config::parse_config;
use cli::types::{JobId, PackageDescriptor, PackageType};

fn main() {
    env_logger::init();

    let app = load_yaml!(".conf/cli.yaml");
    let matches = App::from_yaml(app).get_matches();

    let config_file = matches.value_of("config").unwrap_or("settings.yaml");
    log::debug!("Reading config from {}", config_file);

    let config = parse_config(config_file).unwrap_or_else(|err| {
        log::error!("Failed to parse config: {:?}", err);
        process::exit(-1);
    });

    let mut api = PhylumApi::new(&config.connection.uri);
    log::debug!("Authenticating...");
    let resp = api.authenticate(&config.connection.user, &config.connection.pass).unwrap_or_else(|err| {
        exit(err, "Error attempting to authenticate", -1);
    });
    log::debug!("==> {:?}", resp);

    // If a package was explicitly passed on the command line,
    //  include that.
    if let Some(matches) = matches.subcommand_matches("submit") {
        // If any packages were listed in the config file, include
        //  those as well.
        let mut packages = config.packages.unwrap_or(vec![]);
        // These are required options, so `unwrap` is ok
        let name = matches.value_of("name").unwrap().to_string();
        let version = matches.value_of("version").unwrap().to_string();
        let pkg_type = matches.value_of("type").unwrap_or("npm");
        let pkg_type = PackageType::from_str(pkg_type);
        if let Ok(pkg_type) = pkg_type {
            packages.push(PackageDescriptor { name, version, pkg_type });
        }

        log::info!("Submitting request...");
        let resp = api.submit_request(packages).unwrap_or_else(|err| {
            exit(err, "Error submitting package", -2)
        });
        log::info!("Response => {:?}", resp);
    } else if let Some(matches) = matches.subcommand_matches("status") {
        let request_id = matches.value_of("request_id").unwrap().to_string();
        let request_id = JobId::from_str(&request_id).unwrap_or_else(|err| {
            exit(err, "Received invalid request id", -3)
        });
        let resp = api.poll_status(request_id);
        log::info!("==> {:?}", resp);
    }
}

fn exit(error: impl Error, message: &str, code: i32 ) -> ! {
    log::error!("{}: {:?}", message, error);
    process::exit(code);
}