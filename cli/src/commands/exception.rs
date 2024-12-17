//! Subcommand `phylum exception`.

use std::borrow::Cow;
use std::str::FromStr;

use clap::ArgMatches;
use console::Term;
use dialoguer::{FuzzySelect, Input};
use indexmap::IndexSet;
use purl::{PackageType, Purl};

use crate::api::PhylumApi;
use crate::commands::{CommandResult, ExitCode};
use crate::config::Config;
use crate::format::Format;
use crate::print_user_success;
use crate::spinner::Spinner;
use crate::types::{
    FirewallAction, FirewallLogFilter, IgnoredIssue, IgnoredPackage, Preferences, Suppression,
};

/// Maximum number of package names or versions proposed for exceptions.
const MAX_SUGGESTIONS: usize = 25;

/// Handle `phylum exception` subcommand.
pub async fn handle_exception(
    api: &PhylumApi,
    matches: &ArgMatches,
    config: Config,
) -> CommandResult {
    match matches.subcommand() {
        Some(("list", matches)) => handle_list(api, matches, config).await,
        Some(("add", matches)) => handle_add(api, matches, config).await,
        Some(("remove", matches)) => handle_remove(api, matches, config).await,
        _ => unreachable!("invalid clap configuration"),
    }
}

/// Handle `phylum exception list` subcommand.
pub async fn handle_list(api: &PhylumApi, matches: &ArgMatches, config: Config) -> CommandResult {
    let group = matches.get_one::<String>("group");
    let org = config.org();

    let exceptions = match matches.get_one::<String>("project") {
        Some(project_name) => {
            let group = group.map(String::as_str);
            let project_id = api.get_project_id(project_name, org, group).await?.to_string();
            api.project_preferences(&project_id).await?
        },
        None => api.group_preferences(config.org(), group.unwrap()).await?,
    };

    let pretty = !matches.get_flag("json");
    exceptions.write_stdout(pretty);

    Ok(ExitCode::Ok)
}

/// Handle `phylum exception add` subcommand.
pub async fn handle_add(api: &PhylumApi, matches: &ArgMatches, config: Config) -> CommandResult {
    let no_suggestions = matches.get_flag("no-suggestions");
    let ecosystem = matches.get_one::<String>("ecosystem");
    let project = matches.get_one::<String>("project");
    let version = matches.get_one::<String>("version");
    let reason = matches.get_one::<String>("reason");
    let group = matches.get_one::<String>("group");
    let name = matches.get_one::<String>("name");
    let purl = matches.get_one::<String>("purl");
    let org = config.org();

    // Parse PURL argument or assemble it from its components.
    let mut purl = match purl {
        Some(purl) => Purl::from_str(purl)?,
        None => purl_from_components(api, ecosystem, name, group, org, !no_suggestions).await?,
    };

    // TODO: Show short description of the issues for each version.
    //  => Create issue.
    //  => Maybe also provide full list while prompting for reason?
    //
    // Get suggested versions from Aviary if no argument was supplied.
    let version = purl.version().or(version.map(String::as_str));
    let mut suggested_versions = IndexSet::new();
    if let Some(group) = group.filter(|_| !no_suggestions && version.is_none()) {
        let spinner = Spinner::new();

        let filter = FirewallLogFilter {
            ecosystem: Some(*purl.package_type()),
            action: Some(FirewallAction::AnalysisFailure),
            namespace: purl.namespace(),
            name: Some(purl.name()),
            ..Default::default()
        };
        if let Ok(logs) = api.firewall_log(org, group, filter).await {
            suggested_versions = logs.data.into_iter().map(|log| log.package.version).collect();
        }

        spinner.stop().await;
    }

    // Prompt for version if it wasn't supplied as an argument.
    let version = match version {
        Some(version) => version.to_string().into(),
        None => prompt_version(&suggested_versions)?,
    };
    purl = purl.into_builder().with_version(version).build()?;

    // Prompt for exception reason. if it wasn't supplied as an argument.
    let reason = match reason {
        Some(reason) => reason.into(),
        None => prompt_reason()?,
    };

    // Build suppression API object.
    let suppressions = vec![Suppression::Package(IgnoredPackage {
        purl: Cow::Owned(purl.to_string()),
        reason: Cow::Borrowed(&reason),
    })];

    match project {
        Some(project_name) => {
            let group = group.map(String::as_str);
            let project_id = api.get_project_id(project_name, org, group).await?.to_string();
            api.project_suppress(&project_id, &suppressions).await?;
        },
        None => api.group_suppress(org, group.unwrap(), &suppressions).await?,
    }

    print_user_success!("Successfully added suppression for {}", purl.to_string());

    Ok(ExitCode::Ok)
}

/// Handle `phylum exception remove` subcommand.
pub async fn handle_remove(api: &PhylumApi, matches: &ArgMatches, config: Config) -> CommandResult {
    let ecosystem = matches.get_one::<String>("ecosystem");
    let project = matches.get_one::<String>("project");
    let version = matches.get_one::<String>("version");
    let group = matches.get_one::<String>("group");
    let name = matches.get_one::<String>("name");
    let purl = matches.get_one::<String>("purl");
    let tag = matches.get_one::<String>("tag");
    let id = matches.get_one::<String>("id");
    let org = config.org();

    let mut exceptions = match matches.get_one::<String>("project") {
        Some(project_name) => {
            let group = group.map(String::as_str);
            let project_id = api.get_project_id(project_name, org, group).await?.to_string();
            api.project_preferences(&project_id).await?
        },
        None => api.group_preferences(config.org(), group.unwrap()).await?,
    };

    // Filter issue suppressions with CLI args.
    if tag.is_some() || id.is_some() {
        exceptions.ignored_issues.retain(|issue| {
            id.is_none_or(|id| id == &issue.id) && tag.is_none_or(|tag| tag == &issue.tag)
        });
    }

    // Filter package suppressions with CLI args.
    if ecosystem.is_some() || name.is_some() || version.is_some() || purl.is_some() {
        let purl = purl.map(|purl| Purl::from_str(purl));
        let (ecosystem, namespace, name, version) = match purl {
            Some(Ok(ref purl)) => {
                let package_type = Cow::Owned(purl.package_type().to_string());
                (Some(package_type), purl.namespace(), Some(purl.name()), purl.version())
            },
            Some(Err(err)) => return Err(err.into()),
            None => (
                ecosystem.map(Cow::Borrowed),
                None,
                name.map(String::as_str),
                version.map(String::as_str),
            ),
        };

        exceptions.ignored_packages.retain(|pkg| {
            let purl = match Purl::from_str(&pkg.purl) {
                Ok(purl) => purl,
                Err(_) => return false,
            };

            let package_type = purl.package_type().to_string();
            ecosystem.as_ref().is_none_or(|ecosystem| **ecosystem == package_type)
                && namespace.is_none_or(|namespace| Some(namespace) == purl.namespace())
                && name.is_none_or(|name| name == purl.name())
                && version.is_none_or(|version| Some(version) == purl.version())
        });
    }

    let unsuppressions = vec![prompt_removal(&exceptions)?];

    match project {
        Some(project_name) => {
            let group = group.map(String::as_str);
            let project_id = api.get_project_id(project_name, org, group).await?.to_string();
            api.project_unsuppress(&project_id, &unsuppressions).await?;
        },
        None => api.group_unsuppress(org, group.unwrap(), &unsuppressions).await?,
    }

    match &unsuppressions[0] {
        Suppression::Package(IgnoredPackage { purl, .. }) => {
            print_user_success!("Successfully removed suppression for package {purl:?}");
        },
        Suppression::Issue(IgnoredIssue { id, tag, .. }) => {
            print_user_success!("Successfully removed suppression for issue {tag:?} [{id}]");
        },
    }

    Ok(ExitCode::Ok)
}

/// Creat a PURL from its individual components.
async fn purl_from_components(
    api: &PhylumApi,
    cli_ecosystem: Option<&String>,
    cli_name: Option<&String>,
    group: Option<&String>,
    org: Option<&str>,
    suggestions: bool,
) -> anyhow::Result<Purl> {
    // Prompt for ecosystem if it wasn't supplied as an argument.
    let ecosystem = match cli_ecosystem {
        Some(ecosystem) => ecosystem.into(),
        None => Cow::Owned(prompt_ecosystem()?),
    };
    let ecosystem = PackageType::from_str(&ecosystem).unwrap();

    // Get suggested names from Aviary if no argument was supplied.
    let mut suggested_names: IndexSet<Purl> = IndexSet::new();
    if let Some(group) = group.filter(|_| suggestions && cli_name.is_none()) {
        let spinner = Spinner::new();

        let filter = FirewallLogFilter {
            ecosystem: Some(ecosystem),
            action: Some(FirewallAction::AnalysisFailure),
            ..Default::default()
        };
        if let Ok(logs) = api.firewall_log(org, group, filter).await {
            for log in logs.data {
                let purl = Purl::builder(ecosystem, log.package.name)
                    .with_namespace(log.package.namespace)
                    .build()?;
                suggested_names.insert(purl);
            }
        }

        spinner.stop().await;
    }

    // Prompt for name if it wasn't supplied as an argument.
    let purl = match cli_name {
        Some(name) => Purl::builder_with_combined_name(ecosystem, name).build()?,
        None => prompt_name(ecosystem, &suggested_names)?,
    };

    Ok(purl)
}

/// Ask for an ecosystem.
fn prompt_ecosystem() -> dialoguer::Result<String> {
    let ecosystems = ["cargo", "gem", "golang", "maven", "npm", "nuget", "pypi"];

    let prompt = "[ENTER] Select and Confirm\nSelect ecosystem";
    let index = FuzzySelect::new().with_prompt(prompt).items(&ecosystems).interact()?;

    println!();

    Ok(ecosystems[index].to_owned())
}

/// Ask for a package name.
fn prompt_name(ecosystem: PackageType, suggestions: &'_ IndexSet<Purl>) -> anyhow::Result<Purl> {
    // Get space available for suggestions.
    let term_size = Term::stdout().size_checked().unwrap_or((u16::MAX, u16::MAX));
    let max_suggestions = (term_size.0 as usize - 3).min(MAX_SUGGESTIONS);

    let mut prompt = "[ENTER] Confirm\nSpecify package name";

    // Suggest possible names.
    if !suggestions.is_empty() {
        prompt = "[ENTER] Confirm\nEnter number or specify package name";

        for (i, suggestion) in suggestions.iter().take(max_suggestions).enumerate().rev() {
            println!("({i}) {}", suggestion.combined_name());
        }
        println!();
    }

    let input: String = Input::new().with_prompt(prompt).interact_text()?;

    let purl = match usize::from_str(&input) {
        Ok(index) if index < suggestions.len() && index < MAX_SUGGESTIONS => {
            suggestions[index].clone()
        },
        _ => Purl::builder_with_combined_name(ecosystem, &input).build()?,
    };

    println!("Using package {}\n", purl.combined_name());

    Ok(purl)
}

/// Ask for a package version.
fn prompt_version(suggestions: &'_ IndexSet<String>) -> dialoguer::Result<Cow<'_, str>> {
    // Get space available for suggestions.
    let term_size = Term::stdout().size_checked().unwrap_or((u16::MAX, u16::MAX));
    let max_suggestions = (term_size.0 as usize - 3).min(MAX_SUGGESTIONS);

    let mut prompt = "[ENTER] Confirm\nSpecify package version";

    // Suggest possible names.
    if !suggestions.is_empty() {
        prompt = "[ENTER] Confirm\nEnter number or specify package name";

        for (i, suggestion) in suggestions.iter().take(max_suggestions).enumerate().rev() {
            println!("({i}) {suggestion}");
        }
        println!();
    }

    let input: String = Input::new().with_prompt(prompt).interact_text()?;

    let version: Cow<'_, str> = match usize::from_str(&input) {
        Ok(index) if index < suggestions.len() && index < MAX_SUGGESTIONS => {
            Cow::Borrowed(&suggestions[index])
        },
        _ => Cow::Owned(input),
    };

    println!("Using version {version:?}\n");

    Ok(version)
}

/// Ask for suppression reason.
fn prompt_reason() -> dialoguer::Result<String> {
    let prompt = "[ENTER] Confirm\nEnter reason for this exception";
    let reason: String = Input::new().with_prompt(prompt).interact_text()?;
    println!("Using reason {reason:?}\n");
    Ok(reason)
}

/// Ask for suppression reason.
fn prompt_removal<'a>(preferences: &'a Preferences<'a>) -> dialoguer::Result<Suppression<'a>> {
    let ignored_packages = preferences.ignored_packages.iter().map(|pkg| Cow::Borrowed(&*pkg.purl));
    let ignored_issues = preferences
        .ignored_issues
        .iter()
        .map(|issue| Cow::Owned(format!("[{}] {}", issue.tag, issue.id)));
    let exceptions: Vec<_> = ignored_packages.chain(ignored_issues).collect();

    let prompt = "[ENTER] Select and Confirm\nSelect exception";
    let index = FuzzySelect::new().with_prompt(prompt).items(&exceptions).interact()?;

    println!();

    match index.checked_sub(preferences.ignored_packages.len()) {
        Some(index) => Ok(Suppression::from(&preferences.ignored_issues[index])),
        None => Ok(Suppression::from(&preferences.ignored_packages[index])),
    }
}
