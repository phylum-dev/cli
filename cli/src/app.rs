use clap::builder::PossibleValuesParser;
use clap::{Arg, ArgAction, ArgGroup, Command, ValueHint};
use git_version::git_version;
use lazy_static::lazy_static;

#[cfg(feature = "extensions")]
use crate::commands::extensions;
use crate::commands::parse;

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
            Arg::new("no-config")
                .long("no-config")
                .help("Ignore all configuration files")
                .conflicts_with("config")
                .action(ArgAction::SetTrue),
            Arg::new("timeout")
                .short('t')
                .long("timeout")
                .value_name("TIMEOUT")
                .help("Set the timeout (in seconds) for requests to the Phylum api"),
            Arg::new("ignore-certs")
                .action(ArgAction::SetTrue)
                .long("ignore-certs")
                .alias("no-check-certificate")
                .help("Don't validate the server certificate when performing api requests"),
            Arg::new("org")
                .short('o')
                .long("org")
                .value_name("ORG")
                .help("Phylum organization")
                .global(true),
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

    #[cfg(feature = "extensions")]
    {
        app = extensions::add_extensions_subcommands(app);
    }

    app
}

/// Add non-extension subcommands.
pub fn add_subcommands(command: Command) -> Command {
    #[allow(unused_mut)]
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
                    .value_name("PROJECT_NAME")
                    .help("Project to be queried"),
                Arg::new("group")
                    .short('g')
                    .long("group")
                    .value_name("GROUP_NAME")
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
                    Command::new("status").about("Get current project information").args(&[
                        Arg::new("json")
                            .action(ArgAction::SetTrue)
                            .short('j')
                            .long("json")
                            .help("Produce output in json format (default: false)"),
                        Arg::new("project")
                            .short('p')
                            .long("project")
                            .value_name("PROJECT_NAME")
                            .help("Specify a project to use for analysis"),
                        Arg::new("group")
                            .short('g')
                            .long("group")
                            .value_name("GROUP_NAME")
                            .help("Specify a group to use for analysis")
                            .requires("project"),
                    ]),
                )
                .subcommand(
                    Command::new("create").about("Create a new project").args(&[
                        Arg::new("name")
                            .value_name("NAME")
                            .help("Name of the project")
                            .required(true),
                        Arg::new("group")
                            .short('g')
                            .long("group")
                            .value_name("GROUP_NAME")
                            .help("Group which will be the owner of the project"),
                        Arg::new("repository-url")
                            .short('r')
                            .long("repository-url")
                            .value_name("repository_url")
                            .help("Repository URL of the project"),
                    ]),
                )
                .subcommand(
                    Command::new("delete").about("Delete a project").aliases(["rm"]).args(&[
                        Arg::new("name")
                            .value_name("NAME")
                            .help("Name of the project")
                            .required(true),
                        Arg::new("group")
                            .short('g')
                            .long("group")
                            .value_name("GROUP_NAME")
                            .help("Group that owns the project"),
                    ]),
                )
                .subcommand(
                    Command::new("update").about("Update a project").args(&[
                        Arg::new("project-id")
                            .short('i')
                            .long("project-id")
                            .value_name("PROJECT_ID")
                            .help("ID of the project to be updated"),
                        Arg::new("group")
                            .short('g')
                            .long("group")
                            .value_name("GROUP_NAME")
                            .help("Group that owns the project"),
                        Arg::new("name")
                            .short('n')
                            .long("name")
                            .value_name("NAME")
                            .help("New project name"),
                        Arg::new("repository-url")
                            .short('r')
                            .long("repository-url")
                            .value_name("REPOSITORY_URL")
                            .help("New repository URL"),
                        Arg::new("default-label")
                            .short('l')
                            .long("default-label")
                            .help("Default project label"),
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
                            .value_name("GROUP_NAME")
                            .help("Group to list projects for"),
                        Arg::new("no-group")
                            .action(ArgAction::SetTrue)
                            .long("no-group")
                            .help("Exclude all group projects from the output")
                            .conflicts_with("group"),
                    ]),
                )
                .subcommand(
                    Command::new("link").about("Link a repository to a project").args(&[
                        Arg::new("name")
                            .value_name("NAME")
                            .help("Name of the project")
                            .required(true),
                        Arg::new("group")
                            .short('g')
                            .long("group")
                            .value_name("GROUP_NAME")
                            .help("Group owning the project"),
                    ]),
                ),
        )
        .subcommand(
            Command::new("package").about("Retrieve the details of a specific package").args(&[
                Arg::new("package-type")
                    .index(1)
                    .value_name("TYPE")
                    .help("Package ecosystem type")
                    .value_parser(["npm", "rubygems", "pypi", "maven", "nuget", "golang", "cargo"])
                    .required(true),
                Arg::new("name")
                    .index(2)
                    .value_name("NAME")
                    .help("The name of the package.")
                    .required(true),
                Arg::new("version")
                    .index(3)
                    .value_name("VERSION")
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
                    .value_name("FILTER")
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
                )
                .subcommand(
                    Command::new("create-token").about("Create a new API token").args(&[
                        Arg::new("token-name")
                            .value_name("TOKEN_NAME")
                            .action(ArgAction::Set)
                            .required(true)
                            .help("Unique name to identify the new token"),
                        Arg::new("expiry")
                            .value_name("DAYS")
                            .short('e')
                            .long("expiry")
                            .action(ArgAction::Set)
                            .help("Number of days the token will be valid"),
                    ]),
                ),
        )
        .subcommand(Command::new("ping").about("Ping the remote system to verify it is available"))
        .subcommand(
            Command::new("parse")
                .about("Parse dependency files and output their packages as JSON")
                .args(&[
                    Arg::new("depfile")
                        .value_name("DEPENDENCY_FILE")
                        .value_hint(ValueHint::FilePath)
                        .help("Path to the dependency file to parse")
                        .action(ArgAction::Append),
                    Arg::new("type")
                        .short('t')
                        .long("type")
                        .value_name("TYPE")
                        .requires("depfile")
                        .help("Dependency file type used for all lockfiles (default: auto)")
                        .value_parser(PossibleValuesParser::new(parse::lockfile_types(true))),
                    Arg::new("skip-sandbox")
                        .action(ArgAction::SetTrue)
                        .long("skip-sandbox")
                        .help("Run lockfile generation without sandbox protection"),
                    Arg::new("no-generation")
                        .action(ArgAction::SetTrue)
                        .long("no-generation")
                        .help("Disable generation of lockfiles from manifests"),
                ]),
        )
        .subcommand(
            Command::new("analyze")
                .about("Submit a request for analysis to the processing system")
                .args(&[
                    Arg::new("label")
                        .short('l')
                        .long("label")
                        .value_name("LABEL")
                        .help("Specify a label to use for analysis"),
                    Arg::new("json")
                        .action(ArgAction::SetTrue)
                        .short('j')
                        .long("json")
                        .help("Produce output in json format (default: false)"),
                    Arg::new("project")
                        .short('p')
                        .long("project")
                        .value_name("PROJECT_NAME")
                        .help("Specify a project to use for analysis"),
                    Arg::new("group")
                        .short('g')
                        .long("group")
                        .value_name("GROUP_NAME")
                        .help("Specify a group to use for analysis")
                        .requires("project"),
                    Arg::new("depfile")
                        .value_name("DEPENDENCY_FILE")
                        .value_hint(ValueHint::FilePath)
                        .help("Path to the dependency file to submit")
                        .action(ArgAction::Append),
                    Arg::new("type")
                        .short('t')
                        .long("type")
                        .value_name("TYPE")
                        .requires("depfile")
                        .help("Dependency file type used for all lockfiles (default: auto)")
                        .value_parser(PossibleValuesParser::new(parse::lockfile_types(true))),
                    Arg::new("base")
                        .short('b')
                        .long("base")
                        .value_name("FILE")
                        .value_hint(ValueHint::FilePath)
                        .help("Previous list of dependencies for analyzing the delta")
                        .hide(true),
                    Arg::new("skip-sandbox")
                        .action(ArgAction::SetTrue)
                        .long("skip-sandbox")
                        .help("Run lockfile generation without sandbox protection"),
                    Arg::new("no-generation")
                        .action(ArgAction::SetTrue)
                        .long("no-generation")
                        .help("Disable generation of lockfiles from manifests"),
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
                            .value_name("GROUP_NAME")
                            .help("Name for the new group")
                            .required(true),
                    ),
                )
                .subcommand(
                    Command::new("delete").about("Delete a group").arg(
                        Arg::new("group_name")
                            .value_name("GROUP_NAME")
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
                            .help("Group to manage the members for")
                            .required(true)])
                        .arg_required_else_help(true)
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
                Arg::new("depfile")
                    .short('d')
                    .long("dependency-file")
                    .value_name("DEPENDENCY_FILE")
                    .help("Project-relative dependency file path")
                    .action(ArgAction::Append),
                Arg::new("type")
                    .short('t')
                    .long("type")
                    .value_name("TYPE")
                    .requires("depfile")
                    .help("Dependency file type used for all lockfiles (default: auto)")
                    .value_parser(PossibleValuesParser::new(parse::lockfile_types(true))),
                Arg::new("force")
                    .short('f')
                    .long("force")
                    .help("Overwrite existing configurations without confirmation")
                    .action(ArgAction::SetTrue),
                Arg::new("repository-url")
                    .short('r')
                    .long("repository-url")
                    .value_name("REPOSITORY_URL")
                    .help("Repository URL of the project"),
            ]),
        )
        .subcommand(
            Command::new("status").about("Get Phylum project details").args(&[Arg::new("json")
                .action(ArgAction::SetTrue)
                .short('j')
                .long("json")
                .help("Produce output in json format (default: false)")]),
        )
        .subcommand(
            Command::new("find-dependency-files")
                .about("Find all lockfile and manifest paths")
                .hide(true),
        )
        .subcommand(
            Command::new("org")
                .about("Manage organizations")
                .arg_required_else_help(true)
                .subcommand_required(true)
                .subcommand(
                    Command::new("list")
                        .about("List all organizations the user is a member of")
                        .args(&[Arg::new("json")
                            .action(ArgAction::SetTrue)
                            .short('j')
                            .long("json")
                            .help("Produce output in json format (default: false)")]),
                )
                .subcommand(
                    Command::new("member")
                        .about("Manage organization members")
                        .arg_required_else_help(true)
                        .subcommand_required(true)
                        .subcommand(
                            Command::new("list").about("List organization members").args(&[
                                Arg::new("json")
                                    .action(ArgAction::SetTrue)
                                    .short('j')
                                    .long("json")
                                    .help("Produce member list in json format (default: false)"),
                            ]),
                        )
                        .subcommand(
                            Command::new("add").about("Add user to organization").args(&[
                                Arg::new("user")
                                    .value_name("USER")
                                    .help("User(s) to be added")
                                    .action(ArgAction::Append)
                                    .required(true),
                            ]),
                        )
                        .subcommand(
                            Command::new("remove")
                                .alias("rm")
                                .about("Remove user from organization")
                                .args(&[Arg::new("user")
                                    .value_name("USER")
                                    .help("User(s) to be removed")
                                    .action(ArgAction::Append)
                                    .required(true)]),
                        ),
                )
                .subcommand(
                    Command::new("link")
                        .about("Select an organization as default for all operations")
                        .args(&[Arg::new("org")
                            .value_name("ORG")
                            .help("Organization to use as default")]),
                )
                .subcommand(
                    Command::new("unlink").about("Clear the configured default organization"),
                ),
        )
        .subcommand(
            Command::new("firewall")
                .about("Manage the package firewall")
                .arg_required_else_help(true)
                .subcommand_required(true)
                .subcommand(
                    Command::new("log").about("Show firewall activity log").args(&[
                        Arg::new("json")
                            .action(ArgAction::SetTrue)
                            .short('j')
                            .long("json")
                            .help("Produce output in json format (default: false)"),
                        Arg::new("group")
                            .value_name("GROUP_NAME")
                            .help("Firewall group to list log activity for")
                            .required(true),
                        Arg::new("package-type")
                            .long("package-type")
                            .value_name("PACKAGE_TYPE")
                            .help("Only show logs matching this package type")
                            .value_parser([
                                "npm", "gem", "pypi", "maven", "nuget", "golang", "cargo",
                            ]),
                        Arg::new("purl")
                            .long("purl")
                            .value_name("PURL")
                            .help("Only show logs matching this PURL")
                            .conflicts_with("package-type"),
                        Arg::new("action")
                            .long("action")
                            .value_name("ACTION")
                            .help("Only show logs matching this log action")
                            .value_parser([
                                "Download",
                                "AnalysisSuccess",
                                "AnalysisFailure",
                                "AnalysisWarning",
                            ]),
                        Arg::new("before")
                            .long("before")
                            .value_name("TIMESTAMP")
                            .help("Only show logs created before this timestamp (RFC3339 format)"),
                        Arg::new("after")
                            .long("after")
                            .value_name("TIMESTAMP")
                            .help("Only show logs created after this timestamp (RFC3339 format)"),
                        Arg::new("limit")
                            .long("limit")
                            .value_name("COUNT")
                            .help("Maximum number of log entries to show")
                            .default_value("10")
                            .value_parser(1..=10_000),
                    ]),
                ),
        )
        .subcommand(
            Command::new("exception")
                .about("Manage analysis exceptions")
                .arg_required_else_help(true)
                .subcommand_required(true)
                .subcommand(
                    Command::new("list")
                        .about("List active analysis exceptions")
                        .group(ArgGroup::new("subject").args(["group", "project"]).required(true))
                        .args(&[
                            Arg::new("json")
                                .action(ArgAction::SetTrue)
                                .short('j')
                                .long("json")
                                .help("Produce output in json format (default: false)"),
                            Arg::new("group")
                                .short('g')
                                .long("group")
                                .value_name("GROUP_NAME")
                                .help("Group to list exceptions for"),
                            Arg::new("project")
                                .short('p')
                                .long("project")
                                .value_name("PROJECT_NAME")
                                .help("Project to list exceptions for"),
                        ]),
                )
                .subcommand(
                    Command::new("add")
                        .about("Add a new analysis exception")
                        .group(ArgGroup::new("subject").args(["group", "project"]).required(true))
                        .args(&[
                            Arg::new("group")
                                .short('g')
                                .long("group")
                                .value_name("GROUP_NAME")
                                .help("Group to add exception to"),
                            Arg::new("project")
                                .short('p')
                                .long("project")
                                .value_name("PROJECT_NAME")
                                .help("Project to add exceptions to"),
                            Arg::new("package-type")
                                .long("package-type")
                                .value_name("PACKAGE_TYPE")
                                .help("Package type of the package to add an exception for")
                                .value_parser([
                                    "npm", "gem", "pypi", "maven", "nuget", "golang", "cargo",
                                ]),
                            Arg::new("name")
                                .short('n')
                                .long("name")
                                .value_name("PACKAGE_NAME")
                                .help(
                                    "Fully qualified name of the package to add an exception for",
                                ),
                            Arg::new("version")
                                .long("version")
                                .value_name("VERSION")
                                .help("Version of the package to add an exception for"),
                            Arg::new("purl")
                                .long("purl")
                                .value_name("PURL")
                                .help("Package in PURL format")
                                .conflicts_with_all(["package-type", "name", "version"]),
                            Arg::new("reason")
                                .short('r')
                                .long("reason")
                                .value_name("REASON")
                                .help("Reason for adding this exception"),
                            Arg::new("no-suggestions")
                                .short('s')
                                .long("no-suggestions")
                                .action(ArgAction::SetTrue)
                                .help("Do not query package firewall to make suggestions"),
                        ]),
                )
                .subcommand(
                    Command::new("remove")
                        .about("Remove an existing analysis exception")
                        .group(ArgGroup::new("subject").args(["group", "project"]).required(true))
                        .group(
                            ArgGroup::new("package")
                                .args(["package-type", "name", "version", "purl"])
                                .conflicts_with("issue"),
                        )
                        .group(ArgGroup::new("issue").args(["id", "tag"]))
                        .args(&[
                            Arg::new("group")
                                .short('g')
                                .long("group")
                                .value_name("GROUP_NAME")
                                .help("Group to remove exception from"),
                            Arg::new("project")
                                .short('p')
                                .long("project")
                                .value_name("PROJECT_NAME")
                                .help("Project to remove exceptions from"),
                            Arg::new("package-type")
                                .long("package-type")
                                .value_name("PACKAGE_TYPE")
                                .help("Package type of the exception which should be removed")
                                .value_parser([
                                    "npm", "gem", "pypi", "maven", "nuget", "golang", "cargo",
                                ]),
                            Arg::new("name")
                                .short('n')
                                .long("name")
                                .value_name("PACKAGE_NAME")
                                .help(
                                    "Fully qualified package name of the exception which should \
                                     be removed",
                                ),
                            Arg::new("version")
                                .long("version")
                                .value_name("VERSION")
                                .help("Package version of the exception which should be removed"),
                            Arg::new("purl")
                                .long("purl")
                                .value_name("PURL")
                                .help("Package in PURL format")
                                .conflicts_with_all(["package-type", "name", "version"]),
                            Arg::new("id")
                                .long("id")
                                .value_name("ISSUE_ID")
                                .help("Issue ID of the exception which should be removed"),
                            Arg::new("tag")
                                .long("tag")
                                .value_name("ISSUE_TAG")
                                .help("Issue tag of the exception which should be removed"),
                        ]),
                ),
        );

    #[cfg(feature = "extensions")]
    {
        app = app.subcommand(extensions::command());
    }

    #[cfg(unix)]
    {
        app = app
            .subcommand(
                Command::new("sandbox").hide(true).about("Run an application in a sandbox").args(
                    &[
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
                        Arg::new("cmd")
                            .help("Command to be executed")
                            .value_name("CMD")
                            .required(true),
                        Arg::new("args")
                            .help("Command arguments")
                            .value_name("ARG")
                            .trailing_var_arg(true)
                            .allow_hyphen_values(true)
                            .action(ArgAction::Append),
                    ],
                ),
            )
            .subcommand(
                Command::new("parse-sandboxed")
                    .args(&[
                        Arg::new("depfile")
                            .value_name("DEPENDENCY_FILE")
                            .required(true)
                            .help("Canonical dependency file path"),
                        Arg::new("display-path")
                            .value_name("DISPLAY_PATH")
                            .required(true)
                            .help("Dependency file display path"),
                        Arg::new("type")
                            .long("type")
                            .value_name("TYPE")
                            .help("Dependency file type used (default: auto)")
                            .value_parser(PossibleValuesParser::new(parse::lockfile_types(true))),
                        Arg::new("generate-lockfile")
                            .long("generate-lockfile")
                            .help("Whether lockfile generation should be performed")
                            .action(ArgAction::SetTrue),
                        Arg::new("skip-sandbox")
                            .long("skip-sandbox")
                            .help("Skip sandbox initialization")
                            .action(ArgAction::SetTrue),
                    ])
                    .about("Run lockfile generation inside sandbox and write it to STDOUT")
                    .hide(true),
            );
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
