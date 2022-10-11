use clap::builder::PossibleValuesParser;
use clap::{Arg, ArgAction, Command, ValueHint};
use git_version::git_version;
use lazy_static::lazy_static;

use crate::commands::{extensions, parse};

const VERSION: &str = git_version!(args = ["--dirty=-modified", "--tags"], cargo_prefix = "cargo:");

lazy_static! {
    pub static ref USER_AGENT: String = format!("{}/{}", env!("CARGO_PKG_NAME"), VERSION);
}

const FILTER_ABOUT: &str = r#"Provide a filter used to limit the issues displayed

EXAMPLES
# Show only issues with severity of at least 'high'
    --filter=high

# Show issues with severity of 'critical' in the 'author'
and 'engineering' domains
    --filter=crit,aut,eng
"#;

pub fn app() -> Command {
    // NOTE: We do not use the `arg!` macro here since it causes a stack overflow on
    // Windows.
    #[allow(unused_mut)]
    let mut app = Command::new("phylum")
        .bin_name("phylum")
        .version(VERSION)
        .author("Phylum, Inc.")
        .about("Client interface to the Phylum system")
        .next_display_order(None)
        .args(&[
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Sets a custom config file")
                .value_hint(ValueHint::FilePath),
            Arg::new("timeout")
                .short('t')
                .long("timeout")
                .value_name("TIMEOUT")
                .help("Set the timeout (in seconds) for requests to the Phylum api"),
            Arg::new("no-check-certificate")
                .action(ArgAction::SetTrue)
                .long("no-check-certificate")
                .help("Don't validate the server certificate when performing api requests"),
            Arg::new("verbose")
                .short('v')
                .global(true)
                .help("Increase the level of verbosity (the maximum is -vvv)")
                .action(ArgAction::Count),
            Arg::new("quiet")
                .short('q')
                .global(true)
                .help("Reduce the level of verbosity (the maximum is -qq)")
                .action(ArgAction::Count)
                .conflicts_with("verbose"),
        ])
        .subcommand(
            Command::new("update")
                .about("Check for a new release of the Phylum CLI tool and update if one exists")
                .arg(
                    Arg::new("prerelease")
                        .action(ArgAction::SetTrue)
                        .short('p')
                        .long("prerelease")
                        .help("Update to the latest prerelease (vs. stable, default: false)")
                        .hide(true),
                ),
        )
        .subcommand(
            Command::new("history").about("Return information about historical jobs").args(&[
                Arg::new("JOB_ID")
                    .value_name("JOB_ID")
                    .help("The job id to query (or `current` for the most recent job)"),
                Arg::new("verbose")
                    .action(ArgAction::SetTrue)
                    .short('v')
                    .long("verbose")
                    .help("Increase verbosity of api response."),
                Arg::new("filter").long("filter").value_name("filter").help(FILTER_ABOUT),
                Arg::new("json")
                    .action(ArgAction::SetTrue)
                    .short('j')
                    .long("json")
                    .help("Produce output in json format (default: false)"),
                Arg::new("project")
                    .short('p')
                    .long("project")
                    .value_name("project_name")
                    .help("Project name used to filter jobs"),
            ]),
        )
        .subcommand(
            Command::new("project")
                .about("Create, list, link and set thresholds for projects")
                .args(&[
                    Arg::new("json")
                        .action(ArgAction::SetTrue)
                        .short('j')
                        .long("json")
                        .help("Produce output in json format (default: false)"),
                    Arg::new("group")
                        .short('g')
                        .long("group")
                        .value_name("group_name")
                        .help("Group to list projects for"),
                ])
                .aliases(&["projects"])
                .subcommand(
                    Command::new("create").about("Create a new project").args(&[
                        Arg::new("name")
                            .value_name("name")
                            .help("Name of the project")
                            .required(true),
                        Arg::new("group")
                            .short('g')
                            .long("group")
                            .value_name("group_name")
                            .help("Group which will be the owner of the project"),
                    ]),
                )
                .subcommand(
                    Command::new("delete").about("Delete a project").aliases(&["rm"]).args(&[
                        Arg::new("name")
                            .value_name("name")
                            .help("Name of the project")
                            .required(true),
                        Arg::new("group")
                            .short('g')
                            .long("group")
                            .value_name("group_name")
                            .help("Group that owns the project"),
                    ]),
                )
                .subcommand(
                    Command::new("list").about("List all existing projects").args(&[
                        Arg::new("json")
                            .action(ArgAction::SetTrue)
                            .short('j')
                            .long("json")
                            .help("Produce output in json format (default: false)"),
                        Arg::new("group")
                            .short('g')
                            .long("group")
                            .value_name("group_name")
                            .help("Group to list projects for"),
                    ]),
                )
                .subcommand(
                    Command::new("link").about("Link a repository to a project").args(&[
                        Arg::new("name")
                            .value_name("name")
                            .help("Name of the project")
                            .required(true),
                        Arg::new("group")
                            .short('g')
                            .long("group")
                            .value_name("group_name")
                            .help("Group owning the project"),
                    ]),
                )
                .subcommand(
                    Command::new("set-thresholds")
                        .about("Interactively set risk domain thresholds for a project")
                        .args(&[
                            Arg::new("name")
                                .value_name("name")
                                .help("Name of the project")
                                .required(true),
                            Arg::new("group")
                                .short('g')
                                .long("group")
                                .value_name("group_name")
                                .help("Group owning the project"),
                        ]),
                ),
        )
        .subcommand(
            Command::new("package").about("Retrieve the details of a specific package").args(&[
                Arg::new("name").value_name("name").help("The name of the package.").required(true),
                Arg::new("version")
                    .action(ArgAction::SetTrue)
                    .value_name("version")
                    .help("The version of the package.")
                    .required(true),
                Arg::new("package-type")
                    .short('t')
                    .long("package-type")
                    .value_name("type")
                    .help("The type of the package (\"npm\", \"ruby\", \"pypi\", etc.)"),
                Arg::new("json")
                    .action(ArgAction::SetTrue)
                    .short('j')
                    .long("json")
                    .help("Produce output in json format (default: false)"),
                Arg::new("filter").long("filter").value_name("filter").help(FILTER_ABOUT),
            ]),
        )
        .subcommand(
            Command::new("auth")
                .about("Manage authentication, registration, and API keys")
                .subcommand(Command::new("register").about("Register a new account"))
                .subcommand(Command::new("login").about("Login to an existing account"))
                .subcommand(
                    Command::new("status").about("Return the current authentication status"),
                )
                .subcommand(
                    Command::new("token").about("Return the current authentication token").arg(
                        Arg::new("bearer")
                            .action(ArgAction::SetTrue)
                            .short('b')
                            .long("bearer")
                            .help("Output the short-lived bearer token for the Phylum API"),
                    ),
                ),
        )
        .subcommand(Command::new("ping").about("Ping the remote system to verify it is available"))
        .subcommand(
            Command::new("parse").about("Parse a lockfile").args(&[
                Arg::new("LOCKFILE")
                    .value_name("LOCKFILE")
                    .value_hint(ValueHint::FilePath)
                    .help("The package lock file to submit.")
                    .required(true),
                Arg::new("lockfile-type")
                    .short('t')
                    .long("lockfile-type")
                    .value_name("type")
                    .help("The type of the lock file (default: auto)")
                    .value_parser(PossibleValuesParser::new(parse::lockfile_types())),
            ]),
        )
        .subcommand(
            Command::new("analyze")
                .about("Submit a request for analysis to the processing system")
                .args(&[
                    Arg::new("LOCKFILE")
                        .value_name("LOCKFILE")
                        .value_hint(ValueHint::FilePath)
                        .help("The package lock file to submit.")
                        .required(true),
                    Arg::new("force").action(ArgAction::SetTrue).short('F').long("force").help(
                        "Force re-processing of packages (even if they already exist in the \
                         system)",
                    ),
                    Arg::new("label").short('l').value_name("label"),
                    Arg::new("verbose")
                        .action(ArgAction::SetTrue)
                        .short('v')
                        .long("verbose")
                        .help("Increase verbosity of api response."),
                    Arg::new("filter").long("filter").value_name("filter").help(FILTER_ABOUT),
                    Arg::new("json")
                        .action(ArgAction::SetTrue)
                        .short('j')
                        .long("json")
                        .help("Produce output in json format (default: false)"),
                    Arg::new("project")
                        .short('p')
                        .long("project")
                        .value_name("project_name")
                        .help("Specify a project to use for analysis"),
                    Arg::new("group")
                        .short('g')
                        .long("group")
                        .value_name("group_name")
                        .help("Specify a group to use for analysis")
                        .requires("project"),
                ]),
        )
        .subcommand(
            Command::new("batch")
                .hide(true)
                .about("Submits a batch of requests to the processing system")
                .args(&[
                    Arg::new("file")
                        .short('f')
                        .long("file")
                        .value_name("file")
                        .help(
                            "File (or piped stdin) containing the list of packages (format \
                             `<name>:<version>`)",
                        )
                        .value_hint(ValueHint::FilePath),
                    Arg::new("type")
                        .short('t')
                        .long("type")
                        .value_name("type")
                        .help("Package type (`npm`, `rubygems`, `pypi`, etc)"),
                    Arg::new("force").action(ArgAction::SetTrue).short('F').long("force").help(
                        "Force re-processing of packages (even if they already exist in the \
                         system)",
                    ),
                    Arg::new("low-priority")
                        .action(ArgAction::SetTrue)
                        .short('L')
                        .long("low-priority"),
                    Arg::new("label").short('l').long("label"),
                    Arg::new("project")
                        .short('p')
                        .long("project")
                        .value_name("project_name")
                        .help("Project to use for analysis"),
                    Arg::new("group")
                        .short('g')
                        .long("group")
                        .value_name("group_name")
                        .help("Group to use for analysis")
                        .requires("project"),
                ]),
        )
        .subcommand(Command::new("version").about("Display application version"))
        .subcommand(
            Command::new("group")
                .about("Interact with user groups")
                .arg(
                    Arg::new("json")
                        .action(ArgAction::SetTrue)
                        .short('j')
                        .long("json")
                        .help("Produce group list in json format (default: false)"),
                )
                .subcommand(
                    Command::new("list").about("List all groups the user is a member of").arg(
                        Arg::new("json")
                            .action(ArgAction::SetTrue)
                            .short('j')
                            .long("json")
                            .help("Produce output in json format (default: false)"),
                    ),
                )
                .subcommand(
                    Command::new("create").about("Create a new group").arg(
                        Arg::new("group_name")
                            .value_name("group_name")
                            .help("Name for the new group")
                            .required(true),
                    ),
                ),
        )
        .subcommand(extensions::command());

    app = extensions::add_extensions_subcommands(app);

    #[cfg(feature = "selfmanage")]
    {
        app = app.subcommand(
            Command::new("uninstall").about("Uninstall the Phylum CLI").arg(
                Arg::new("purge")
                    .action(ArgAction::SetTrue)
                    .short('p')
                    .long("purge")
                    .help("Remove all files, including configuration files (default: false)"),
            ),
        );
    }

    app
}
