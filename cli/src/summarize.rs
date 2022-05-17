use std::fmt;
use std::str::FromStr;

use ansi_term::Color::*;
use chrono::NaiveDateTime;
use color::Color;
use phylum_types::types::group::ListUserGroupsResponse;
use phylum_types::types::job::{
    AllJobsStatusResponse, CancelJobResponse, JobDescriptor, JobStatusResponse,
};
use phylum_types::types::package::*;
use phylum_types::types::project::*;
use prettytable::*;

use crate::filter::Filter;
use crate::print::{self, table_format};
use crate::render::Renderable;

#[derive(Debug)]
pub struct Histogram {
    min: f64,
    max: f64,
    bins: usize,
    values: Vec<usize>,
}

impl Histogram {
    fn new(data: &[f64], min: f64, max: f64, bins: usize) -> Histogram {
        let mut values: Vec<usize> = vec![0; bins];

        let step = (max - min) / bins as f64;
        let scale = 100.0;

        for &y in data.iter() {
            if y < min || y > max {
                continue;
            }

            let mut bucket_id = ((y * scale).floor() / (step * scale)) as usize;

            // Account for packages with a "perfect" (i.e. 1.0) score
            // This is generally unlikely but possible with packages that have
            //  not yet had analytics run on them
            // Also account for scores on the edge 10, 20, 30...
            if y != 0.0 && (y * 100.0) % 10.0 == 0.0 {
                bucket_id -= 1;
            }

            if bucket_id < values.len() {
                values[bucket_id as usize] += 1;
            }
        }
        Histogram {
            min,
            max,
            bins,
            values,
        }
    }

    fn buckets(&self) -> Vec<(f64, f64)> {
        let step = (self.max - self.min) / self.bins as f64;
        let mut buckets: Vec<(f64, f64)> = Vec::new();

        let mut acc = self.min;
        while acc < self.max {
            buckets.push((acc, acc + step));
            acc += step;
        }
        buckets.pop();
        buckets
    }
}

impl fmt::Display for Histogram {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let scale = |s| {
            let max = *self.values.iter().max().unwrap_or(&1) as f32;
            56.0 * f32::log2(s) / f32::log2(max)
        };

        let output = self
            .values
            .iter()
            .rev()
            .zip(self.buckets().iter().rev())
            .fold("".to_string(), |acc, x| {
                let min = (100.0 * x.1 .0).round() as u32;
                vec![
                    acc,
                    format!(
                        "{:>4} - {:<4} [{:>5}] {}",
                        match min {
                            0 => min,
                            _ => min + 1,
                        },
                        (100.0 * x.1 .1).round() as u32,
                        x.0,
                        "█".repeat(scale(*x.0 as f32) as usize)
                    ),
                ]
                .join("\n")
            });

        write!(f, "{:^10} {:>8}", "Score", "Count")?;
        write!(f, "{}", output)
    }
}

pub trait Summarize: Renderable {
    fn summarize(&self, _filter: Option<Filter>) {
        println!("{}", self.render());
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

fn response_to_table<T>(resp: &JobStatusResponse<T>) -> Table
where
    T: Scored,
{
    let ecosystem = PackageType::from_str(&resp.ecosystem).unwrap_or(PackageType::Npm);

    let date_time = NaiveDateTime::from_timestamp(resp.created_at / 1000, 0);

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
            format!("{} UTC", date_time),
        ),
        (
            "Num Deps",
            resp.packages.len().to_string(),
            "Job ID",
            resp.job_id.to_string(),
        ),
        (
            "User ID",
            resp.user_email.to_string(),
            "View in Phylum UI",
            format!("https://app.phylum.io/projects/{}", resp.project),
        ),
    ];
    let mut summary = details.iter().fold("".to_string(), |acc, x| {
        format!("{}\n{:>16}: {:<36} {:>24}: {:<36}", acc, x.0, x.1, x.2, x.3)
    });
    summary = format!("{}\n       Ecosystem: {}", summary, ecosystem.render());

    let status = if resp.num_incomplete > 0 {
        format!("{:>16}: {}", "Status", Yellow.paint("INCOMPLETE"))
    } else if resp.pass {
        format!("{:>16}: {}", "Status", Green.paint("PASS"))
    } else {
        format!(
            "{:>16}: {}\n{:>16}: {}",
            "Status",
            Red.paint("FAIL"),
            "Reason",
            resp.msg
        )
    };

    let scores: Vec<f64> = resp.packages.iter().map(|p| p.score()).collect();

    let hist = Histogram::new(scores.as_slice(), 0.0, 1.0, 10);

    let mut histogram_table = table!([hist.to_string(), resp.thresholds.render()]);
    histogram_table.set_format(table_format(1, 8));

    let mut table = Table::new();
    table.add_row(row![summary]);

    if resp.num_incomplete > 0 {
        let notice = format!(
            "\n{}: {:.2}% of submitted packages are currently being processed. Scores may change once processing completes.\n            For more information on processing visit https://docs.phylum.io/docs/processing.",
            Purple.paint("PROCESSING"),
            (resp.num_incomplete as f32/resp.packages.len() as f32)*100.0
        );
        table.add_row(row![notice]);
    }

    table.add_row(row![histogram_table]);
    table.add_row(row![status]);
    table.set_format(table_format(0, 0));
    table
}

impl Summarize for JobStatusResponse<PackageStatus> {
    fn summarize(&self, _filter: Option<Filter>) {
        let t: Table = response_to_table(self);
        t.printstd();
    }
}

fn check_filter_issue(filter: &Filter, issue: &Issue) -> bool {
    let mut include = true;
    if let Some(ref level) = filter.level {
        if issue.risk_level < *level {
            include = false;
        }
    }
    if let Some(ref domains) = filter.domains {
        if !domains.contains(&issue.risk_domain) {
            include = false;
        }
    }
    include
}

impl Summarize for JobStatusResponse<PackageStatusExtended> {
    fn summarize(&self, filter: Option<Filter>) {
        let table_1: Table = response_to_table(self);

        let mut table_2 = Table::new();
        table_2.set_format(table_format(3, 1));

        let mut issues: Vec<&Issue> = vec![];

        for p in &self.packages {
            for issue in &p.issues {
                if let Some(ref filter) = filter {
                    if check_filter_issue(filter, issue) {
                        issues.push(issue);
                    }
                } else {
                    issues.push(issue);
                }
            }
        }

        issues.sort_by(|a, b| a.risk_level.partial_cmp(&b.risk_level).unwrap());
        issues.reverse();

        for issue in issues {
            let rows: Vec<Row> = issue_to_row(issue);
            for r in rows {
                table_2.add_row(r);
            }
            table_2.add_empty_row();
        }

        table_1.printstd();
        table_2.printstd();
    }
}

impl Summarize for PackageStatusExtended {
    fn summarize(&self, filter: Option<Filter>) {
        let mut issues_table = Table::new();
        issues_table.set_format(table_format(3, 0));

        let issues = if let Some(ref filter) = filter {
            self.issues
                .iter()
                .filter_map(|i| {
                    let mut include = true;

                    if let Some(ref level) = filter.level {
                        if i.risk_level < *level {
                            include = false;
                        }
                    }

                    if let Some(domains) = &filter.domains {
                        if !domains.contains(&i.risk_domain) {
                            include = false;
                        }
                    }
                    if include {
                        Some(i.to_owned())
                    } else {
                        None
                    }
                })
                .collect::<Vec<Issue>>()
        } else {
            self.issues.to_owned()
        };

        for issue in &issues {
            let rows: Vec<Row> = issue_to_row(issue);
            for mut row in rows {
                row.remove_cell(2);
                issues_table.add_row(row);
            }
            issues_table.add_empty_row();
        }

        let risk_to_string = |key| {
            format!(
                "{}",
                (100.0 * self.risk_vectors.get(key).unwrap_or(&1.0)).round()
            )
        };

        let mut risks_table = table![
            ["Author Risk:", r -> risk_to_string("author")],
            ["Engineering Risk:", r -> risk_to_string("engineering")],
            ["License Risk:", r -> risk_to_string("license")],
            ["Malicious Code Risk:", r -> risk_to_string("malicious_code")],
            ["Vulnerability Risk:", r -> risk_to_string("vulnerability")]
        ];
        risks_table.set_format(table_format(3, 1));

        println!("{}", self.render());

        println!(" Risk Vectors:");
        risks_table.printstd();

        if !issues_table.is_empty() {
            println!("\n Issues:");
            issues_table.printstd();
        }
    }
}

impl Summarize for Vec<JobDescriptor> {
    fn summarize(&self, _filter: Option<Filter>) {
        println!("Last {} runs\n\n{}", self.len(), self.render());
    }
}

impl<T> Summarize for Vec<T> where T: Renderable {}

impl Summarize for String {}
impl Summarize for PackageStatus {}
impl Summarize for ProjectDetailsResponse {}
impl Summarize for AllJobsStatusResponse {}
impl Summarize for CancelJobResponse {}
impl Summarize for ListUserGroupsResponse {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_check() {
        let filter_string = "lic";
        let filter = Filter::from_str(filter_string).expect("Failed to parse filter string: {}");

        let issue = r#"{
                    "title": "Commercial license risk in xmlrpc@0.3.0",
                    "description": "license is medium risk",
                    "risk_level": "medium",
                    "domain": "license"
                    }"#;
        let issue: Issue = serde_json::from_str(issue).unwrap();

        let include = check_filter_issue(&filter, &issue);
        assert!(include);

        let filter_string = "mal";
        let filter = Filter::from_str(filter_string).expect("Failed to parse filter string: {}");
        let include = check_filter_issue(&filter, &issue);
        assert!(!include);
    }
}

fn risk_level_to_color(level: &RiskLevel) -> Color {
    match level {
        RiskLevel::Critical => color::BRIGHT_RED,
        RiskLevel::High => color::YELLOW,
        RiskLevel::Medium => color::BRIGHT_YELLOW,
        RiskLevel::Low => color::BLUE,
        RiskLevel::Info => color::WHITE,
    }
}

fn issue_to_row(issue: &Issue) -> Vec<Row> {
    let row_1 = Row::new(vec![
        Cell::new_align(&issue.risk_level.to_string(), format::Alignment::LEFT).with_style(
            Attr::ForegroundColor(risk_level_to_color(&issue.risk_level)),
        ),
        Cell::new_align(
            &format!("{} [{}]", &issue.title, issue.risk_domain),
            format::Alignment::LEFT,
        )
        .with_style(Attr::Bold),
    ]);

    let row_2 = Row::new(vec![
        Cell::new(""),
        Cell::new(&textwrap::fill(&issue.description, 80)),
        Cell::new(""),
    ]);

    vec![row_1, row![], row_2]
}
