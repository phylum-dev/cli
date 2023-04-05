use std::cmp;
use std::io::{self, Write};
use std::str::{self, FromStr};

use chrono::{DateTime, NaiveDateTime, Utc};
use console::style;
use phylum_types::types::group::{
    GroupMember, ListGroupMembersResponse, ListUserGroupsResponse, UserGroup,
};
use phylum_types::types::job::{AllJobsStatusResponse, JobDescriptor, JobStatusResponse};
use phylum_types::types::package::{
    Issue, IssuesListItem, Package, PackageStatus, PackageStatusExtended, PackageType, RiskLevel,
};
use phylum_types::types::project::ProjectSummaryResponse;
use prettytable::format::Alignment;
use prettytable::{color as table_color, row, table, Attr, Cell, Row, Table};
use serde::Serialize;
use unicode_width::UnicodeWidthStr;

use crate::histogram::Histogram;
use crate::print::{self, table_format};
use crate::types::{HistoryJob, PolicyEvaluationResponse};

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

    /// Output to stdout.
    fn write_stdout(&self, pretty: bool) {
        if pretty {
            self.pretty(&mut io::stdout());
        } else {
            self.json(&mut io::stdout());
        }
    }
}

impl Format for PolicyEvaluationResponse {
    fn pretty<W: Write>(&self, writer: &mut W) {
        let _ = writeln!(writer, "{}", self.report);
    }
}

impl Format for Vec<ProjectSummaryResponse> {
    fn pretty<W: Write>(&self, writer: &mut W) {
        // Maximum length of the name column.
        const MAX_NAME_WIDTH: usize = 28;

        let table = format_table::<fn(&ProjectSummaryResponse) -> String, _>(self, &[
            ("Project Name", |project| print::truncate(&project.name, MAX_NAME_WIDTH).into_owned()),
            ("Project ID", |project| project.id.to_string()),
        ]);
        let _ = writeln!(writer, "{table}");
    }
}

impl Format for ListUserGroupsResponse {
    fn pretty<W: Write>(&self, writer: &mut W) {
        // Maximum length of group name column.
        const MAX_NAME_WIDTH: usize = 25;
        // Maximum length of owner email column.
        const MAX_OWNER_WIDTH: usize = 25;

        let table = format_table::<fn(&UserGroup) -> String, _>(&self.groups, &[
            ("Group Name", |group| print::truncate(&group.group_name, MAX_NAME_WIDTH).into_owned()),
            ("Owner", |group| print::truncate(&group.owner_email, MAX_OWNER_WIDTH).into_owned()),
            ("Creation Time", |group| group.created_at.format("%FT%RZ").to_string()),
        ]);
        let _ = writeln!(writer, "{table}");
    }
}

impl Format for ListGroupMembersResponse {
    fn pretty<W: Write>(&self, writer: &mut W) {
        // Maximum length of email column.
        const MAX_EMAIL_WIDTH: usize = 25;
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
            let status = if job.num_incomplete > 0 {
                style("INCOMPLETE").yellow().to_string()
            } else if !job.pass {
                style("FAIL").red().to_string()
            } else {
                style("PASS").green().to_string()
            };

            let date = job
                .date
                .parse::<DateTime<Utc>>()
                .map(|date| date.format("%FT%RZ").to_string())
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
            let rows: Vec<Row> = issueslistitem_to_row(issue);
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

impl Format for JobStatusResponse<PackageStatus> {
    fn pretty<W: Write>(&self, writer: &mut W) {
        let table = response_to_table(self);
        let _ = writeln!(writer, "{table}");
    }
}

impl Format for JobStatusResponse<PackageStatusExtended> {
    fn pretty<W: Write>(&self, writer: &mut W) {
        let table_1: Table = response_to_table(self);

        let mut table_2 = Table::new();
        table_2.set_format(table_format(3, 1));

        let mut issues: Vec<&Issue> = self
            .packages
            .iter()
            .flat_map(|package| package.issues.iter().map(|issue| &issue.issue))
            .collect();

        issues.sort_by(|a, b| a.severity.partial_cmp(&b.severity).unwrap());
        issues.reverse();

        for issue in issues {
            let rows: Vec<Row> = issue_to_row(issue);
            for r in rows {
                table_2.add_row(r);
            }
            table_2.add_empty_row();
        }

        let _ = writeln!(writer, "{table_1}");
        let _ = writeln!(writer, "{table_2}");
    }
}

impl Format for Vec<HistoryJob> {
    fn pretty<W: Write>(&self, writer: &mut W) {
        let table = format_table::<fn(&HistoryJob) -> String, _>(self, &[
            ("Job ID", |job| job.id.clone()),
            ("Label", |job| job.label.clone()),
            ("Creation Time", |job| job.created.format("%FT%RZ").to_string()),
        ]);
        let _ = writeln!(writer, "{table}");
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

fn issueslistitem_to_row(issue: &IssuesListItem) -> Vec<Row> {
    let row_1 = Row::new(vec![
        Cell::new_align(&issue.impact.to_string(), Alignment::LEFT)
            .with_style(Attr::ForegroundColor(risk_level_to_color(&issue.impact))),
        Cell::new_align(&format!("{} [{}]", &issue.title, issue.risk_type), Alignment::LEFT)
            .with_style(Attr::Bold),
    ]);

    let row_2 = Row::new(vec![
        Cell::new(""),
        Cell::new(&textwrap::fill(&issue.description, 80)),
        Cell::new(""),
    ]);

    vec![row_1, row![], row_2]
}

fn response_to_table<T>(resp: &JobStatusResponse<T>) -> Table
where
    T: Scored,
{
    // Combine all ecosystems into a comma-separated string.
    let ecosystems = resp
        .ecosystems
        .iter()
        .flat_map(|ecosystem| {
            Some(match PackageType::from_str(ecosystem).ok()? {
                PackageType::Npm => "NPM",
                PackageType::RubyGems => "RubyGems",
                PackageType::PyPi => "PyPI",
                PackageType::Maven => "Maven",
                PackageType::Nuget => "NuGet",
                PackageType::Golang => "Golang",
                PackageType::Cargo => "Cargo",
            })
        })
        .collect::<Vec<_>>();
    let mut ecosystems_value = ecosystems.join(", ");

    // Add fallback if no ecosystem could be identified.
    if ecosystems_value.is_empty() {
        ecosystems_value = "Unknown".into();
    }

    // Perform correct pluralization for ecosystems title.
    let ecosystems_title = if ecosystems.len() >= 2 { "Ecosystems" } else { "Ecosystem" };

    let date_time =
        NaiveDateTime::from_timestamp_opt(resp.created_at / 1000, 0).unwrap_or_default();

    let details = [
        (
            "Project",
            print::truncate(&resp.project_name, 36).to_string(),
            "Label",
            resp.label.as_ref().unwrap_or(&"".to_string()).to_owned(),
        ),
        (
            "Proj Score",
            (100.0 * resp.score).round().to_string(),
            "Date",
            format!("{date_time} UTC"),
        ),
        ("Num Deps", resp.packages.len().to_string(), "Job ID", resp.job_id.to_string()),
    ];
    let mut summary = details.iter().fold("".to_string(), |acc, x| {
        format!("{}\n{:>16}: {:<36} {:>24}: {:<36}", acc, x.0, x.1, x.2, x.3)
    });
    summary = format!("{summary}\n{ecosystems_title:>16}: {ecosystems_value}");

    let status = if resp.num_incomplete > 0 {
        format!("{:>16}: {}", "Status", style("INCOMPLETE").yellow())
    } else if resp.pass {
        format!("{:>16}: {}", "Status", style("PASS").green())
    } else {
        format!("{:>16}: {}\n{:>16}: {}", "Status", style("FAIL").red(), "Reason", resp.msg)
    };

    let scores: Vec<f64> = resp.packages.iter().map(|p| p.score()).collect();

    let hist = Histogram::new(scores.as_slice(), 0.0, 1.0, 10);

    let normalize = |t: f32| (t * 100.0).round() as u32;
    let mut thresholds_table = table!(
        [r => "Thresholds:"],
        [r => "Project Score:", normalize(resp.thresholds.total)],
        [r => "Malicious Code Risk MAL:", normalize(resp.thresholds.malicious)],
        [r => "Vulnerability Risk VLN:", normalize(resp.thresholds.vulnerability)],
        [r => "Engineering Risk ENG:", normalize(resp.thresholds.engineering)],
        [r => "Author Risk AUT:", normalize(resp.thresholds.author)],
        [r => "License Risk LIC:", normalize(resp.thresholds.license)]
    );
    thresholds_table.set_format(table_format(0, 0));

    let mut histogram_table = table!([hist.to_string(), thresholds_table.to_string()]);
    histogram_table.set_format(table_format(1, 8));

    let mut table = Table::new();
    table.add_row(row![summary]);

    if resp.num_incomplete > 0 {
        let notice = format!(
            "\n{}: {:.2}% of submitted packages are currently being processed. Scores may change once processing completes.\n            For more information on processing visit https://docs.phylum.io/docs/processing.",
            style("PROCESSING").magenta(),
            (resp.num_incomplete as f32/resp.packages.len() as f32)*100.0
        );
        table.add_row(row![notice]);
    }

    table.add_row(row![histogram_table]);
    table.add_row(row![status]);
    table.set_format(table_format(0, 0));
    table
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
