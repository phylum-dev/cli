use clap::{arg, Command, ValueHint};
use git_version::git_version;

const VERSION: &str = git_version!(
    args = ["--dirty=-modified", "--tags"],
    cargo_prefix = "cargo:"
);

const FILTER_ABOUT: &str = r#"Provide a filter used to limit the issues displayed

EXAMPLES
# Show only issues with severity of at least 'high'
    --filter=high

# Show issues with severity of 'critical' in the 'author'
and 'engineering' domains
    --filter=crit,aut,eng
"#;

pub fn app<'a>() -> clap::Command<'a> {
    #[allow(unused_mut)]
    let mut app = Command::new("phylum")
        .bin_name("phylum")
        .version(VERSION)
        .author("Phylum, Inc.")
        .about("Client interface to the Phylum system")
        .args(&[
            arg!(-c --config <FILE> "Sets a custom config file").required(false).value_hint(ValueHint::FilePath),
            arg!(-t --timeout <TIMEOUT> "Set the timeout (in seconds) for requests to the Phylum api").required(false),
            arg!(--"no-check-certificate" "Don't validate the server certificate when performing api requests"),
        ])
        .subcommand(
            Command::new("update")
                .about("Check for a new release of the Phylum CLI tool and update if one exists")
                .arg(arg!(
                    -p --prerelease "Update to the latest prerelease (vs. stable, default: false)"
                ).hide(true))
        )
        .subcommand(
            Command::new("history")
                .about("Return information about historical jobs")
                .args(&[
                    arg!([JOB_ID] "The job id to query (or `current` for the most recent job)"),
                    arg!(-v --verbose "Increase verbosity of api response."),
                    arg!(--filter <filter>).required(false).help(FILTER_ABOUT),
                    arg!(-j --json "Produce output in json format (default: false)"),
                    arg!(-p --project <project_name> "Project name used to filter jobs").required(false),
                ])
                .subcommand(
                    Command::new("project")
                        .about("Show jobs for a specific project (deprecated)")
                        .args(&[
                            arg!(<project_name> "Name of the project").required(false),
                            arg!(<job_id>).required(false).hide(true),
                        ])
                        .hide(true)
                )
        )
        .subcommand(
            Command::new("project")
                .about("Create, list, link and set thresholds for projects")
                .arg(arg!(-j --json "Produce output in json format (default: false)"))
                .aliases(&["projects"])
                .subcommand(
                    Command::new("create")
                        .about("Create a new project")
                        .arg(arg!(<name> "Name of the project"))
                )
                .subcommand(
                    Command::new("list")
                        .about("List all existing projects")
                        .arg(arg!(-j --json "Produce output in json format (default: false)"))
                )
                .subcommand(
                    Command::new("link")
                        .about("Link a repository to a project")
                        .arg(arg!(<name> "Name of the project"))
                )
                .subcommand(
                    Command::new("set-thresholds")
                        .about("Interactively set risk domain thresholds for a project")
                        .arg(arg!(<name> "Name of the project"))
                )
        )
        .subcommand(
            Command::new("package")
                .about("Retrieve the details of a specific package")
                .args(&[
                    arg!(<name> "The name of the package."),
                    arg!(<version> "The version of the package."),
                    arg!(-t --"package-type" <type> "The type of the package (\"npm\", \"ruby\", \"pypi\", etc.)").required(false),
                    arg!(-j --json "Produce output in json format (default: false)")
                ])
        )
        .subcommand(
            Command::new("auth")
                .about("Manage authentication, registration, and API keys")
                .subcommand(Command::new("register").about("Register a new account"))
                .subcommand(Command::new("login").about("Login to an existing account"))
                .subcommand(Command::new("status").about("Return the current authentication status"))
                .subcommand(
                    Command::new("token")
                    .about("Return the current authentication token")
                    .arg(arg!(-b --bearer "Output the short-lived bearer token for the Phylum API"))
                )
        )
        .subcommand(
            Command::new("ping").about("Ping the remote system to verify it is available")
        )
        .subcommand(
            Command::new("analyze")
                .about("Submit a request for analysis to the processing system")
                .args(&[
                    arg!([LOCKFILE] "The package lock file to submit.").value_hint(ValueHint::FilePath),
                    arg!(-F --force "Force re-processing of packages (even if they already exist in the system)"),
                    arg!(-l <label>).required(false),
                    arg!(-v --verbose "Increase verbosity of api response."),
                    arg!(--filter <filter>).required(false).help(FILTER_ABOUT),
                    arg!(-j --json "Produce output in json format (default: false)"),
                    arg!(-p --project <project_name> "Project to use for analysis").required(false),
                ])
        )
        .subcommand(
            Command::new("batch")
                .hide(true)
                .about("Submits a batch of requests to the processing system")
                .args(&[
                    arg!(-f --file <file> "File (or piped stdin) containing the list of packages (format `<name>:<version>`)").required(false).value_hint(ValueHint::FilePath),
                    arg!(-t --type <type> "Package type (`npm`, `rubygems`, `pypi`, etc)").required(false),
                    arg!(-F --force "Force re-processing of packages (even if they already exist in the system)"),
                    arg!(-L --"low-priority"),
                    arg!(-l --label),
                    arg!(-p --project <project_name> "Project to use for analysis").required(false),
                ])
        )
        .subcommand(
            Command::new("version")
                .about("Display application version")
        );

    #[cfg(feature = "selfmanage")]
    {
        app = app.subcommand(
            Command::new("uninstall")
                .about("Uninstall the Phylum CLI")
                .arg(arg!(
                    -p --purge "Remove all files, including configuration files (default: false)"
                )),
        );
    }

    app
}
