use clap::{arg, App, AppSettings};

const FILTER_ABOUT: &'static str = 
r#"Provide a filter used to limit the issues displayed

EXAMPLES
# Show only issues with severity of at least 'high'
    --filter=high

# Show issues with severity of 'critical' in the 'author'
and 'engineering' domains
    --filter=crit,aut,eng
"#;

pub fn app<'a>() -> clap::App<'a> {
    App::new("phylum")
        .bin_name("phylum")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Phylum, Inc.")
        .about("Client interface to the Phylum system")
        .args(&[
            arg!(-c --config <FILE> "Sets a custom config file").required(false),
            arg!(-t --timeout <TIMEOUT> "Set the timeout (in seconds) for requests to the Phylum api").required(false),
            arg!(--"no-check-certificate" "Don't validate the server certificate when performing api requests"),
        ])
        .subcommand(
            App::new("update")
                .about("Check for a new release of the Phylum CLI tool and update if one exists")
                .arg(arg!(
                    -p --prerelease "Update to the latest prerelease (vs. stable, default: false)"
                ))
        )
        .subcommand(
            App::new("history")
                .about("Return information about historical scans")
                .args(&[
                    arg!([JOB_ID] "The job id to query (or `current` for the most recent job)"),
                    arg!(-v --verbose "Increase verbosity of api response."),
                    arg!(--filter <filter>).required(false).help(FILTER_ABOUT),
                    arg!(-j --json "Produce output in json format (default: false)"),
                ])
                .subcommand(
                    App::new("project")
                        .about("Shows a list of projects associated with the user")
                        .args(&[
                            arg!(<project_name>).required(false),
                            arg!(<job_id>).required(false),
                        ])
                )
        )
        .subcommand(
            App::new("projects")
                .about("Create, list, link and set thresholds for projects")
                .subcommand(
                    App::new("create")
                        .about("Create a new project")
                        .arg(arg!(<name>))
                )
                .subcommand(
                    App::new("list")
                        .about("List all existing projects")
                )
                .subcommand(
                    App::new("link")
                        .about("Link a repository to a project")
                        .arg(arg!(<name>))
                )
                .subcommand(
                    App::new("set-thresholds")
                        .about("Set risk domain thresholds for a projects")
                        .arg(arg!(<name>))
                )
        )
        .subcommand(
            App::new("package")
                .about("Retrieve the details of a specific package")
                .args(&[
                    arg!(<name> "The name of the package."),
                    arg!(<version> "The version of the package."),
                    arg!(-t --"package-type" <type> "The type of the package (\"npm\", \"ruby\", \"pypi\", etc.)").required(false),
                    arg!(-j --json "Produce output in json format (default: false)")
                ])
        )
        .subcommand(
            App::new("auth")
                .about("Manage authentication, registration, and API keys")
                .subcommand(App::new("register").about("Register a new account"))
                .subcommand(App::new("login").about("Login to an existing account"))
                .subcommand(App::new("status").about("Return the current authentication status"))
        )
        .subcommand(
            App::new("ping").about("Ping the remote system to verify it is available")
        )
        .subcommand(
            App::new("analyze")
                .about("Submit a request for analysis to the processing system")
                .args(&[
                    arg!([LOCKFILE] "The package lock file to submit."),
                    arg!(-F --force "Force re-processing of packages (even if they already exist in the system)"),
                    arg!(-l <label>).required(false),
                    arg!(-v --verbose "Increase verbosity of api response."),
                    arg!(--filter <filter>).required(false).help(FILTER_ABOUT),
                    arg!(-j --json "Produce output in json format (default: false)"),
                ])
        )
        .subcommand(
            App::new("batch")
                .about("Submits a batch of requests to the processing system")
                .setting(AppSettings::Hidden)
                .args(&[
                    arg!(-f --file <file> "File (or piped stdin) containing the list of packages (format `<name>:<version>`)"),
                    arg!(-t --type <type> "Package type (`npm`, `ruby`, etc)"),
                    arg!(-F --force "Force re-processing of packages (even if they already exist in the system)"),
                    arg!(-L --"low-priority"),
                    arg!(-l --label),
                ])
        )
        .subcommand(
            App::new("version")
                .about("Display application version")
        )
}
