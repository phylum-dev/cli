use clap::builder::PossibleValuesParser;
use clap::{Arg, ArgAction, Command, ValueHint};
use git_version::git_version;
use lazy_static::lazy_static;

use crate::commands::{extensions, parse};

const VERSION: &str = git_version!(args = ["--dirty=-modified", "--tags"], cargo_suffix = "+");

lazy_static! {
    pub static ref USER_AGENT: String = format!("{}/{}", env!("CARGO_PKG_NAME"), VERSION);
}

const FILTER_ABOUT: &str = r#"Provide a filter used to limit the issues displayed

    EXAMPLES:
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
                .long("verbose")
                .global(true)
                .help("Increase the level of verbosity (the maximum is -vvv)")
                .action(ArgAction::Count),
            Arg::new("quiet")
                .short('q')
                .long("quiet")
                .global(true)
                .help("Reduce the level of verbosity (the maximum is -qq)")
                .action(ArgAction::Count)
                .conflicts_with("verbose"),
        ]);

    app = add_subcommands(app);

    app = extensions::add_extensions_subcommands(app);

    app
}

/// Add non-extension subcommands.
pub fn add_subcommands(command: Command) -> Command {
    let mut app = command
        .subcommand(
            Command::new("history").about("Return information about historical jobs").args(&[
                Arg::new("JOB_ID").value_name("JOB_ID").help("The job id to query"),
                Arg::new("json")
                    .action(ArgAction::SetTrue)
                    .short('j')
                    .long("json")
                    .help("Produce output in json format (default: false)"),
                Arg::new("project")
                    .short('p')
                    .long("project")
                    .value_name("project_name")
                    .help("Project to be queried"),
                Arg::new("group")
                    .short('g')
                    .long("group")
                    .value_name("group_name")
                    .help("Group to be queried"),
            ]),
        )
        .subcommand(
            Command::new("project")
                .aliases(["projects"])
                .about("Manage Phylum projects")
                .arg_required_else_help(true)
                .subcommand_required(true)
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
                    Command::new("delete").about("Delete a project").aliases(["rm"]).args(&[
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
                ),
        )
        .subcommand(
            Command::new("package").about("Retrieve the details of a specific package").args(&[
                Arg::new("package-type")
                    .index(1)
                    .value_name("type")
                    .help("Package ecosystem type")
                    .value_parser(["npm", "rubygems", "pypi", "maven", "nuget", "golang", "cargo"])
                    .required(true),
                Arg::new("name")
                    .index(2)
                    .value_name("name")
                    .help("The name of the package.")
                    .required(true),
                Arg::new("version")
                    .index(3)
                    .value_name("version")
                    .help("The version of the package.")
                    .required(true),
                Arg::new("json")
                    .action(ArgAction::SetTrue)
                    .short('j')
                    .long("json")
                    .help("Produce output in json format (default: false)"),
                Arg::new("filter")
                    .short('f')
                    .long("filter")
                    .value_name("filter")
                    .help(FILTER_ABOUT),
            ]),
        )
        .subcommand(
            Command::new("auth")
                .about("Manage authentication, registration, and API keys")
                .arg_required_else_help(true)
                .subcommand_required(true)
                .subcommand(
                    Command::new("register").about("Register a new account").arg(
                        Arg::new("token-name")
                            .action(ArgAction::Set)
                            .short('n')
                            .long("token-name")
                            .help("Unique name for the new token that will be created"),
                    ),
                )
                .subcommand(
                    Command::new("login")
                        .about("Login to an existing account")
                        .arg(
                            Arg::new("reauth")
                                .action(ArgAction::SetTrue)
                                .short('r')
                                .long("reauth")
                                .help("Force a login prompt"),
                        )
                        .arg(
                            Arg::new("token-name")
                                .action(ArgAction::Set)
                                .short('n')
                                .long("token-name")
                                .help("Unique name for the new token that will be created"),
                        ),
                )
                .subcommand(
                    Command::new("status").about("Return the current authentication status"),
                )
                .subcommand(
                    Command::new("set-token").about("Set the current authentication token").arg(
                        Arg::new("token")
                            .value_name("TOKEN")
                            .action(ArgAction::Set)
                            .required(false)
                            .help("Authentication token to store (read from stdin if omitted)"),
                    ),
                )
                .subcommand(
                    Command::new("token").about("Return the current authentication token").arg(
                        Arg::new("bearer")
                            .action(ArgAction::SetTrue)
                            .short('b')
                            .long("bearer")
                            .help("Output the short-lived bearer token for the Phylum API"),
                    ),
                )
                .subcommand(
                    Command::new("list-tokens")
                        .about("List all tokens associated with the logged-in user")
                        .arg(
                            Arg::new("json")
                                .action(ArgAction::SetTrue)
                                .short('j')
                                .long("json")
                                .help("Produce output in json format (default: false)"),
                        ),
                )
                .subcommand(
                    Command::new("revoke-token").about("Revoke an API token").arg(
                        Arg::new("token-name")
                            .value_name("TOKEN_NAME")
                            .action(ArgAction::Append)
                            .help("Unique token names which identify the tokens"),
                    ),
                ),
        )
        .subcommand(Command::new("ping").about("Ping the remote system to verify it is available"))
        .subcommand(
            Command::new("parse").about("Parse lock files and output their packages as JSON").args(
                &[
                    Arg::new("lockfile")
                        .value_name("LOCKFILE")
                        .value_hint(ValueHint::FilePath)
                        .help("The package lock files to submit")
                        .action(ArgAction::Append),
                    Arg::new("lockfile-type")
                        .short('t')
                        .long("lockfile-type")
                        .value_name("type")
                        .requires("lockfile")
                        .help("Lock file type used for all lock files (default: auto)")
                        .value_parser(PossibleValuesParser::new(parse::lockfile_types(true))),
                ],
            ),
        )
        .subcommand(
            Command::new("analyze")
                .about("Submit a request for analysis to the processing system")
                .args(&[
                    Arg::new("label")
                        .short('l')
                        .long("label")
                        .value_name("label")
                        .help("Specify a label to use for analysis"),
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
                    Arg::new("lockfile")
                        .value_name("LOCKFILE")
                        .value_hint(ValueHint::FilePath)
                        .help("The package lock files to submit")
                        .action(ArgAction::Append),
                    Arg::new("lockfile-type")
                        .short('t')
                        .long("lockfile-type")
                        .value_name("type")
                        .requires("lockfile")
                        .help("Lock file type used for all lock files (default: auto)")
                        .value_parser(PossibleValuesParser::new(parse::lockfile_types(true))),
                    Arg::new("base")
                        .short('b')
                        .long("base")
                        .value_name("FILE")
                        .value_hint(ValueHint::FilePath)
                        .help("Previous list of dependencies for analyzing the delta")
                        .hide(true),
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
                        .help("Package ecosystem type")
                        .value_parser([
                            "npm", "rubygems", "pypi", "maven", "nuget", "golang", "cargo",
                        ])
                        .required(true),
                    Arg::new("label").short('l').long("label").help("Label to use for analysis"),
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
                .arg_required_else_help(true)
                .subcommand_required(true)
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
                )
                .subcommand(
                    Command::new("delete").about("Delete a group").arg(
                        Arg::new("group_name")
                            .value_name("group_name")
                            .help("Name for the group to be deleted")
                            .required(true),
                    ),
                )
                .subcommand(
                    Command::new("member")
                        .about("Manage group members")
                        .args(&[Arg::new("group")
                            .short('g')
                            .long("group")
                            .value_name("GROUP")
                            .help("Group to list the members for")
                            .required(true)])
                        .subcommand_required(true)
                        .subcommand(
                            Command::new("list").about("List group members").args(&[Arg::new(
                                "json",
                            )
                            .action(ArgAction::SetTrue)
                            .short('j')
                            .long("json")
                            .help("Produce member list in json format (default: false)")]),
                        )
                        .subcommand(
                            Command::new("add").about("Add user to group").args(&[Arg::new(
                                "user",
                            )
                            .value_name("USER")
                            .help("User(s) to be added")
                            .action(ArgAction::Append)
                            .required(true)]),
                        )
                        .subcommand(
                            Command::new("remove")
                                .alias("rm")
                                .about("Remove user from group")
                                .args(&[Arg::new("user")
                                    .value_name("USER")
                                    .help("User(s) to be removed")
                                    .action(ArgAction::Append)
                                    .required(true)]),
                        ),
                )
                .subcommand(
                    Command::new("transfer").about("Transfer group ownership between users").args(
                        &[
                            Arg::new("group")
                                .short('g')
                                .long("group")
                                .value_name("GROUP")
                                .help("Group to transfer")
                                .required(true),
                            Arg::new("user")
                                .value_name("USER")
                                .help("User the group ownership will be transferred to")
                                .required(true),
                            Arg::new("force")
                                .short('f')
                                .long("force")
                                .help("Do not prompt for confirmation")
                                .action(ArgAction::SetTrue),
                        ],
                    ),
                ),
        )
        .subcommand(
            Command::new("init").about("Setup a new Phylum project").args(&[
                Arg::new("project").value_name("PROJECT_NAME").help("Phylum project name"),
                Arg::new("group")
                    .short('g')
                    .long("group")
                    .value_name("GROUP_NAME")
                    .help("Group which will be the owner of the project"),
                Arg::new("lockfile")
                    .short('l')
                    .long("lockfile")
                    .value_name("LOCKFILE")
                    .help("Project-relative lock file path")
                    .action(ArgAction::Append),
                Arg::new("lockfile-type")
                    .short('t')
                    .long("lockfile-type")
                    .value_name("type")
                    .requires("lockfile")
                    .help("Lock file type used for all lock files (default: auto)")
                    .value_parser(PossibleValuesParser::new(parse::lockfile_types(true))),
                Arg::new("force")
                    .short('f')
                    .long("force")
                    .help("Overwrite existing configurations without confirmation")
                    .action(ArgAction::SetTrue),
            ]),
        )
        .subcommand(
            Command::new("status").about("Get Phylum project details").args(&[Arg::new("json")
                .action(ArgAction::SetTrue)
                .short('j')
                .long("json")
                .help("Produce output in json format (default: false)")]),
        )
        .subcommand(extensions::command());

    #[cfg(unix)]
    {
        app = app.subcommand(
            Command::new("sandbox").hide(true).about("Run an application in a sandbox").args(&[
                Arg::new("allow-read")
                    .help("Add filesystem read sandbox exception")
                    .long("allow-read")
                    .value_name("PATH")
                    .value_hint(ValueHint::FilePath)
                    .action(ArgAction::Append),
                Arg::new("allow-write")
                    .help("Add filesystem write sandbox exception")
                    .long("allow-write")
                    .value_name("PATH")
                    .value_hint(ValueHint::FilePath)
                    .action(ArgAction::Append),
                Arg::new("allow-run")
                    .help("Add filesystem execute sandbox exception")
                    .long("allow-run")
                    .value_name("PATH")
                    .value_hint(ValueHint::FilePath)
                    .action(ArgAction::Append),
                Arg::new("allow-env")
                    .help("Add environment variable access sandbox exception")
                    .long("allow-env")
                    .value_name("ENV_VAR")
                    .num_args(0..=1)
                    .default_missing_value("*")
                    .action(ArgAction::Append),
                Arg::new("allow-net")
                    .help("Add network access sandbox exception")
                    .long("allow-net")
                    .action(ArgAction::SetTrue),
                Arg::new("strict")
                    .help("Do not add any default sandbox exceptions")
                    .long("strict")
                    .action(ArgAction::SetTrue),
                Arg::new("cmd").help("Command to be executed").value_name("CMD").required(true),
                Arg::new("args")
                    .help("Command arguments")
                    .value_name("ARG")
                    .trailing_var_arg(true)
                    .allow_hyphen_values(true)
                    .action(ArgAction::Append),
            ]),
        )
    }

    #[cfg(feature = "selfmanage")]
    {
        app = app
            .subcommand(
                Command::new("uninstall").about("Uninstall the Phylum CLI").arg(
                    Arg::new("purge")
                        .action(ArgAction::SetTrue)
                        .short('p')
                        .long("purge")
                        .help("Remove all files, including configuration files (default: false)"),
                ),
            )
            .subcommand(
                Command::new("update").about("Update to the latest release of the Phylum CLI").arg(
                    Arg::new("prerelease")
                        .action(ArgAction::SetTrue)
                        .short('p')
                        .long("prerelease")
                        .help("Update to the latest prerelease (vs. stable, default: false)")
                        .hide(true),
                ),
            );
    }

    app
}

/// Check if a non-extension subcommand exists.
pub fn is_builtin_subcommand(name: &str) -> bool {
    add_subcommands(Command::new("phylum"))
        .get_subcommands()
        .map(Command::get_name)
        .any(|cmd_name| cmd_name == name)
}
