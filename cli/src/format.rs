#[cfg(feature = "vulnreach")]
use std::collections::HashSet;
use std::io::{self, Write};
use std::{cmp, str};

use chrono::{DateTime, Local, Utc};
use console::style;
use phylum_types::types::group::{GroupMember, ListGroupMembersResponse};
use phylum_types::types::job::{AllJobsStatusResponse, JobDescriptor};
use phylum_types::types::package::{PackageStatus, PackageStatusExtended};
use prettytable::format::Alignment;
use prettytable::{color as table_color, row, table, Attr, Cell, Row, Table};
use serde::Serialize;
use unicode_width::UnicodeWidthStr;
#[cfg(feature = "vulnreach")]
use vulnreach_types::Vulnerability;

use crate::commands::group::ListGroupsEntry;
use crate::commands::status::PhylumStatus;
use crate::print::{self, table_format};
use crate::types::{
    GetProjectResponse, HistoryJob, Issue, OrgMember, OrgMembersResponse, OrgsResponse, Package,
    PolicyEvaluationResponse, PolicyEvaluationResponseRaw, ProjectListEntry, RiskLevel, UserToken,
};

// Maximum length of email column.
const MAX_EMAIL_WIDTH: usize = 25;

/// Format type for CLI output.
pub trait Format: Serialize {
    /// Output JSON format.
    fn json<W: Write>(&self, writer: &mut W) {
        let json = serde_json::to_string_pretty(&self).unwrap_or_else(|e| {
            log::error!("Failed to serialize json response: {}", e);
            "".to_string()
        });
        let _ = writeln!(writer, "{json}");
    }

    /// Output human-friendly format.
    fn pretty<W: Write>(&self, writer: &mut W);

    /// Output human-friendly format with additional information.
    fn pretty_verbose<W: Write>(&self, writer: &mut W) {
        self.pretty(writer);
    }

    /// Output to stdout.
    fn write_stdout(&self, pretty: bool) {
        if pretty {
            self.pretty(&mut io::stdout());
        } else {
            self.json(&mut io::stdout());
        }
    }
}

impl Format for PhylumStatus {
    fn pretty<W: Write>(&self, writer: &mut W) {
        fn write_option<W: Write, T: std::fmt::Display>(
            writer: &mut W,
            label: &str,
            option: Option<T>,
        ) {
            let label = style(label).blue();
            let _ = match option {
                Some(value) => writeln!(writer, "{label}: {}", value),
                None => writeln!(writer, "{label}: {}", style("null").italic().green()),
            };
        }

        let root = self.root.as_ref().map(|root| root.display());

        // Write group fields.
        write_option(writer, "Project", self.project.as_ref());
        write_option(writer, "Group", self.group.as_ref());
        write_option(writer, "Project Root", root);

        // Write known dependency files.
        let depfiles_label = style("Dependency Files").blue();
        if self.dependency_files.is_empty() {
            let _ = writeln!(writer, "{depfiles_label}: {}", style("null").italic().green());
        } else {
            let _ = writeln!(writer, "{depfiles_label}:");
            for depfile in &self.dependency_files {
                let path = depfile.path.display();
                let _ = writeln!(writer, " - {}: {}", style("path").blue(), path);
                let _ = writeln!(writer, "   {}: {}", style("type").blue(), depfile.depfile_type);
            }
        }
    }
}

impl Format for PolicyEvaluationResponse {
    fn pretty<W: Write>(&self, writer: &mut W) {
        let _ = writeln!(writer, "{}", self.report);
    }
}

impl Format for PolicyEvaluationResponseRaw {
    fn pretty<W: Write>(&self, writer: &mut W) {
        let _ = writeln!(writer);

        // Print summary status.
        let status = if self.is_failure {
            style("FAILURE").red()
        } else if self.incomplete_packages_count > 0 {
            style("INCOMPLETE").yellow()
        } else {
            style("SUCCESS").green()
        };
        let _ = writeln!(writer, "Phylum Supply Chain Risk Analysis — {status}\n");

        // Print number of unprocessed packages.
        if self.incomplete_packages_count > 0 {
            let pluralization = if self.incomplete_packages_count == 1 { "" } else { "s" };
            let unprocessed_text =
                format!("{} unprocessed package{}", self.incomplete_packages_count, pluralization);
            let incomplete_message = format!(
                "The analysis contains {}, preventing a complete risk analysis. Phylum is \
                 currently processing these packages and should complete soon. Please wait for up \
                 to 30 minutes, then re-run the analysis.\n",
                style(unprocessed_text).yellow(),
            );
            let _ = writeln!(writer, "{}", textwrap::fill(&incomplete_message, 80));
        }

        // Write summary for each issue.
        for package in self
            .dependencies
            .iter()
            .filter(|package| package.rejections.iter().any(|rejection| !rejection.suppressed))
        {
            let _ = writeln!(writer, "[{}] {}@{}", package.registry, package.name, package.version);

            for rejection in package.rejections.iter().filter(|rejection| !rejection.suppressed) {
                let domain = rejection
                    .source
                    .domain
                    .map_or_else(|| "     ".into(), |domain| format!("[{}]", domain));
                let message = format!("{domain} {}", rejection.title);

                let colored = match rejection.source.severity {
                    Some(RiskLevel::Low) | Some(RiskLevel::Info) => style(message).green(),
                    Some(RiskLevel::Medium) => style(message).yellow(),
                    _ => style(message).red(),
                };

                let _ = writeln!(writer, "  {}", colored);
            }
        }
        if !self.dependencies.is_empty() {
            let _ = writeln!(writer);
        }

        // Print web URI for the job results.
        if let Some(job_link) = &self.job_link {
            let _ = writeln!(writer, "You can find the interactive report here:\n  {}", job_link);
        }
    }
}

impl Format for Vec<ProjectListEntry> {
    fn pretty<W: Write>(&self, writer: &mut W) {
        // Maximum length of the project and group name column.
        const MAX_NAME_WIDTH: usize = 28;

        // Hide group column if no group project is present.
        if self.iter().all(|project| project.group_name.is_none()) {
            let table = format_table::<fn(&ProjectListEntry) -> String, _>(self, &[
                ("Project Name", |project| {
                    print::truncate(&project.name, MAX_NAME_WIDTH).into_owned()
                }),
                ("Project ID", |project| project.id.to_string()),
            ]);
            let _ = writeln!(writer, "{table}");
        } else {
            let table = format_table::<fn(&ProjectListEntry) -> String, _>(self, &[
                ("Group Name", |project| {
                    let group_name = project.group_name.as_deref().unwrap_or("");
                    print::truncate(group_name, MAX_NAME_WIDTH).into_owned()
                }),
                ("Project Name", |project| {
                    print::truncate(&project.name, MAX_NAME_WIDTH).into_owned()
                }),
                ("Project ID", |project| project.id.to_string()),
            ]);
            let _ = writeln!(writer, "{table}");
        }
    }
}

impl Format for GetProjectResponse {
    fn pretty<W: Write>(&self, writer: &mut W) {
        let _ = writeln!(writer, "{}: {}", style("ID").blue(), self.id);
        let _ = writeln!(writer, "{}: {}", style("Name").blue(), self.name);
        if let Some(repository_url) = &self.repository_url {
            let _ = writeln!(writer, "{}: {}", style("Repository URL").blue(), repository_url);
        }
        if let Some(default_label) = &self.default_label {
            let _ = writeln!(writer, "{}: {}", style("Default Label").blue(), default_label);
        }
    }
}

impl Format for Vec<ListGroupsEntry> {
    fn pretty<W: Write>(&self, writer: &mut W) {
        // Maximum length of org and group name columns.
        const MAX_NAME_WIDTH: usize = 25;

        // Skip table formatting with no groups.
        if self.is_empty() {
            let _ = writeln!(
                writer,
                "You don't have any groups yet; create one with `phylum group create <NAME>`."
            );
            return;
        }

        // Use condensed format if only legacy groups are present.
        if self.iter().all(|group| group.org.is_none()) {
            let table = format_table::<fn(&ListGroupsEntry) -> String, _>(self, &[(
                "Group Name",
                |group| print::truncate(&group.name, MAX_NAME_WIDTH).into_owned(),
            )]);
            let _ = writeln!(writer, "{table}");
        } else {
            let table = format_table::<fn(&ListGroupsEntry) -> String, _>(self, &[
                ("Organization Name", |group| match &group.org {
                    Some(org) => print::truncate(org, MAX_NAME_WIDTH).into_owned(),
                    None => String::new(),
                }),
                ("Group Name", |group| print::truncate(&group.name, MAX_NAME_WIDTH).into_owned()),
            ]);
            let _ = writeln!(writer, "{table}");
        }
    }
}

impl Format for ListGroupMembersResponse {
    fn pretty<W: Write>(&self, writer: &mut W) {
        // Maximum length of first name column.
        const MAX_FIRST_NAME_WIDTH: usize = 15;
        // Maximum length of last name column.
        const MAX_LAST_NAME_WIDTH: usize = 15;

        let table = format_table::<fn(&GroupMember) -> String, _>(&self.members, &[
            ("E-Mail", |member| print::truncate(&member.user_email, MAX_EMAIL_WIDTH).into_owned()),
            ("First Name", |member| {
                print::truncate(&member.first_name, MAX_FIRST_NAME_WIDTH).into_owned()
            }),
            ("Last Name", |member| {
                print::truncate(&member.last_name, MAX_LAST_NAME_WIDTH).into_owned()
            }),
        ]);
        let _ = writeln!(writer, "{table}");
    }
}

/// Write object fields
fn write_fields<W: Write>(fields: &[(&str, &str)], writer: &mut W) -> std::io::Result<()> {
    let max_label_width = fields.iter().map(|f| f.0.len()).max().unwrap_or(0);

    for field in fields {
        writeln!(writer, "  {}  {}", style(leftpad(field.0, max_label_width)).blue(), field.1)?;
    }
    Ok(())
}

impl Format for Vec<JobDescriptor> {
    fn pretty<W: Write>(&self, writer: &mut W) {
        let _ = writeln!(writer, "Last {} runs\n", self.len());

        for job in self {
            let status = match (job.num_incomplete, job.pass) {
                (0, false) => style("FAILED").red().to_string(),
                (_, false) => style("INCOMPLETE WITH FAILURE").red().to_string(),
                (0, true) => style("SUCCESS").green().to_string(),
                (_, true) => style("INCOMPLETE").yellow().to_string(),
            };

            let date = job
                .date
                .parse::<DateTime<Utc>>()
                .map(format_datetime)
                .unwrap_or_else(|_| "UNKNOWN".into());

            let _ = writeln!(writer, "Job ID: {}", style(job.job_id).cyan());
            let _ = write_fields(
                &[
                    ("Project Name", &job.project),
                    ("Label", &job.label),
                    ("Creation Time", &date),
                    ("Status", &status),
                    ("Ecosystems", &job.ecosystems.join(",")),
                    ("Dependencies", &job.num_dependencies.to_string()),
                    ("Message", &job.msg),
                ],
                writer,
            );
            let _ = writeln!(writer);
        }
    }
}

impl Format for AllJobsStatusResponse {
    fn pretty<W: Write>(&self, writer: &mut W) {
        let _ = writeln!(writer, "Total jobs: {}", self.total_jobs);
        self.jobs.pretty(writer);
    }
}

impl Format for Package {
    fn pretty<W: Write>(&self, writer: &mut W) {
        let mut issues_table = Table::new();
        issues_table.set_format(table_format(3, 0));

        let issues = self.issues.to_owned();

        for issue in &issues {
            let rows: Vec<Row> = issue_to_row(issue);
            for mut row in rows {
                row.remove_cell(2);
                issues_table.add_row(row);
            }
            issues_table.add_empty_row();
        }

        let risk_to_string = |risk: f32| format!("{}", (100. * risk).round());

        let mut risks_table = table![
            ["Total Risk:", r -> risk_to_string(self.risk_scores.total)],
            ["Author Risk:", r -> risk_to_string(self.risk_scores.author)],
            ["Engineering Risk:", r -> risk_to_string(self.risk_scores.engineering)],
            ["License Risk:", r -> risk_to_string(self.risk_scores.license)],
            ["Malicious Code Risk:", r -> risk_to_string(self.risk_scores.malicious)],
            ["Vulnerability Risk:", r -> risk_to_string(self.risk_scores.vulnerability)]
        ];
        risks_table.set_format(table_format(3, 1));

        let unknown = String::from("Unknown");
        let time = self.published_date.as_ref().unwrap_or(&unknown);

        let mut overview_table = table!(
            ["Package Name:", rB -> self.name, "Package Version:", r -> self.version],
            ["License:", r -> self.license.as_ref().unwrap_or(&"Unknown".to_string()), "Last updated:", r -> time],
            ["Num Deps:", r -> self.dependencies.as_ref().map(Vec::len).unwrap_or(0), "Num Vulns:", r -> self.issues.len()],
            ["Ecosystem:", r -> self.registry]
        );
        overview_table.set_format(table_format(0, 3));
        let _ = writeln!(writer, "{overview_table}");

        let _ = writeln!(writer, " Risk Vectors:");
        risks_table.printstd();

        if !issues_table.is_empty() {
            let _ = writeln!(writer, "\n Issues:");
            issues_table.printstd();
        }
    }
}

impl Format for Vec<HistoryJob> {
    fn pretty<W: Write>(&self, writer: &mut W) {
        let table = format_table::<fn(&HistoryJob) -> String, _>(self, &[
            ("Job ID", |job| job.id.clone()),
            ("Label", |job| job.label.clone().unwrap_or_default()),
            ("Creation Time", |job| format_datetime(job.created)),
        ]);
        let _ = writeln!(writer, "{table}");
    }
}

impl Format for Vec<UserToken> {
    fn pretty<W: Write>(&self, writer: &mut W) {
        // Maximum length of the token name column.
        //
        // We use `47` here since it is the default CLI token length.
        const MAX_TOKEN_WIDTH: usize = 47;

        let table = format_table::<fn(&UserToken) -> String, _>(self, &[
            ("Name", |token| print::truncate(&token.name, MAX_TOKEN_WIDTH).into_owned()),
            ("Creation Time", |token| format_datetime(token.creation_time)),
            ("Access Time", |token| token.access_time.map_or(String::new(), format_datetime)),
            ("Expiry", |token| token.expiry.map_or(String::new(), format_datetime)),
        ]);
        let _ = writeln!(writer, "{table}");
    }
}

impl Format for OrgsResponse {
    fn pretty<W: Write>(&self, writer: &mut W) {
        if self.organizations.is_empty() {
            let _ = writeln!(writer, "User is not part of any organizations");
            return;
        }

        for organization in &self.organizations {
            let _ = writeln!(writer, "{}", organization.name);
        }
    }
}

impl Format for OrgMembersResponse {
    fn pretty<W: Write>(&self, writer: &mut W) {
        let table = format_table::<fn(&OrgMember) -> String, _>(&self.members, &[
            ("E-Mail", |member| print::truncate(&member.email, MAX_EMAIL_WIDTH).into_owned()),
            ("Role", |member| member.role.to_string()),
        ]);
        let _ = writeln!(writer, "{table}");
    }
}

#[cfg(feature = "vulnreach")]
impl Format for Vulnerability {
    fn pretty<W: Write>(&self, writer: &mut W) {
        // Check if any import is calling this vulnerability.
        let affected = if self.vulnerable_dependencies.is_empty() {
            style("unaffected").green()
        } else {
            style("affected").red()
        };

        // Output heading.
        let _ = writeln!(writer, "[{affected}] {} — {}", self.name, self.summary);
    }

    fn pretty_verbose<W: Write>(&self, writer: &mut W) {
        // Print vulnerability summary.
        self.pretty(writer);

        // This should never happen, but skip it just in case.
        if self.vulnerable_dependencies.is_empty() {
            return;
        }

        // Output section heading.
        let _ = writeln!(writer, "Reachability paths:");

        // Filter out duplicate reachability paths.
        let mut unique_paths = HashSet::new();
        for path in &self.vulnerable_dependencies {
            let packages = path.iter().map(|package| &package.name).collect::<Vec<_>>();
            unique_paths.insert(packages);
        }

        // Print dependency paths causing this vulnerability to be reachable.
        let arrow = style("->").blue();
        for path in &unique_paths {
            // This should never happen, but skip it just in case.
            if path.is_empty() {
                continue;
            }

            // Print the callchain as arrow-separated packages.
            let _ = write!(writer, "    {}", path[0]);
            for package in &path[1..] {
                let _ = write!(writer, " {} {}", arrow, package);
            }

            let _ = writeln!(writer);
        }
    }
}

pub trait Scored {
    fn score(&self) -> f64;
}

impl Scored for PackageStatus {
    fn score(&self) -> f64 {
        self.package_score.unwrap_or(1.0)
    }
}

impl Scored for PackageStatusExtended {
    fn score(&self) -> f64 {
        self.basic_status.package_score.unwrap_or(1.0)
    }
}

/// Format any slice into a table.
fn format_table<F, T>(data: &[T], columns: &[(&str, F)]) -> String
where
    F: Fn(&T) -> String,
{
    // Whitespace between the columns.
    const COLUMN_SPACING: usize = 2;

    let mut header = String::new();
    let mut rows = vec![String::new(); data.len()];

    let mut last_column_width = 0;
    for column in columns {
        let mut column_width = column.0.width();

        header = leftpad(&header, last_column_width);
        header.push_str(column.0);

        for i in 0..data.len() {
            let cell = column.1(&data[i]);
            column_width = cmp::max(column_width, cell.width());

            rows[i] = leftpad(&rows[i], last_column_width);
            rows[i].push_str(&cell);
        }

        last_column_width += column_width + COLUMN_SPACING;
    }

    // Color header to distinguish it from rows.
    header = style(header).blue().to_string();

    // Combine header with all rows.
    let rows = rows.join("\n");
    [header, rows].join("\n")
}

/// Leftpad a string with proper unicode width.
fn leftpad(text: &str, width: usize) -> String {
    let delta = width.saturating_sub(text.width());
    format!("{}{}", text, str::repeat(" ", delta))
}

fn issue_to_row(issue: &Issue) -> Vec<Row> {
    let row_1 = Row::new(vec![
        Cell::new_align(&issue.severity.to_string(), Alignment::LEFT)
            .with_style(Attr::ForegroundColor(risk_level_to_color(&issue.severity))),
        Cell::new_align(&format!("{} [{}]", &issue.title, issue.domain), Alignment::LEFT)
            .with_style(Attr::Bold),
    ]);

    let row_2 = Row::new(vec![
        Cell::new(""),
        Cell::new(&textwrap::fill(&issue.description, 80)),
        Cell::new(""),
    ]);

    vec![row_1, row![], row_2]
}

fn risk_level_to_color(level: &RiskLevel) -> table_color::Color {
    match level {
        RiskLevel::Critical => table_color::BRIGHT_RED,
        RiskLevel::High => table_color::YELLOW,
        RiskLevel::Medium => table_color::BRIGHT_YELLOW,
        RiskLevel::Low => table_color::BLUE,
        RiskLevel::Info => table_color::WHITE,
    }
}

/// Convert a UTC timestamp in the local timezone.
fn format_datetime(timestamp: DateTime<Utc>) -> String {
    let local: DateTime<Local> = timestamp.into();

    local.format("%F %R").to_string()
}
