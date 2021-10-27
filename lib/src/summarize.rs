use std::fmt;
use std::str::FromStr;

use ansi_term::Color::{Green, Purple, Red, Yellow};
use chrono::NaiveDateTime;
use prettytable::*;

use crate::filter::Filter;
use crate::render::Renderable;
use crate::types::*;
use crate::utils::table_format;

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

        for &y in data.iter() {
            if y < min || y > max {
                continue;
            }

            let mut bucket_id = ((y - min) / step).round() as usize;

            // Account for packages with a "perfect" (i.e. 1.0) score
            // This is generally unlikely but possible with packages that have
            //  not yet had analytics run on them
            // Also account for scores on the edge 10, 20, 30...
            if y != 0.0 && (y * 100.0) % 10.0 == 0.0 {
                bucket_id -= 1;
            }

            if bucket_id <= values.len() {
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

fn response_to_table<T>(resp: &RequestStatusResponse<T>) -> Table
where
    T: Scored,
{
    let ecosystem = PackageType::from_str(&resp.ecosystem).unwrap_or(PackageType::Npm);

    let date_time = NaiveDateTime::from_timestamp(resp.created_at / 1000, 0);

    let details = [
        (
            "Project",
            resp.project_name.to_string(),
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
            "Type",
            ecosystem.render(),
            "Language",
            ecosystem.language().to_string(),
        ),
        (
            "User ID",
            resp.user_email.to_string(),
            "View in Phylum UI",
            format!("https://app.phylum.io/projects/{}", resp.project),
        ),
    ];
    let summary = details.iter().fold("".to_string(), |acc, x| {
        vec![
            acc,
            format!("{:>16}: {:<36} {:>24}: {:<36}", x.0, x.1, x.2, x.3),
        ]
        .join("\n")
    });

    let status = if resp.num_incomplete > 0 {
        format!(
            "{:>16}: {}",
            "Status",
            Yellow.paint("INCOMPLETE").to_string()
        )
    } else if resp.pass {
        format!("{:>16}: {}", "Status", Green.paint("PASS").to_string())
    } else {
        format!(
            "{:>16}: {}\n{:>16}: {}",
            "Status",
            Red.paint("FAIL").to_string(),
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

impl Summarize for RequestStatusResponse<PackageStatus> {
    fn summarize(&self, _filter: Option<Filter>) {
        let t: Table = response_to_table(self);
        t.printstd();
    }
}

fn vuln_to_rows(
    vuln: &Vulnerability,
    pkg_name: Option<&str>,
    pkg_version: Option<&str>,
) -> Vec<Row> {
    let mut rows = Vec::new();

    let cve_s = if !vuln.cve.is_empty() {
        vuln.cve.join("/")
    } else {
        "[No CVE listed]".to_string()
    };

    let pkg_descriptor = if pkg_name.is_some() && pkg_version.is_some() {
        format!("{}@{}", pkg_name.unwrap(), pkg_version.unwrap())
    } else {
        "".to_string()
    };

    rows.push(Row::new(vec![
        Cell::new_align(&vuln.risk_level.to_string(), format::Alignment::LEFT)
            .with_style(Attr::ForegroundColor(color::Color::from(&vuln.risk_level))),
        Cell::new_align(
            &format!(
                "{} is vulnerable to {} [{}]",
                &pkg_descriptor, vuln.title, cve_s
            ),
            format::Alignment::LEFT,
        )
        .with_style(Attr::Bold),
    ]));
    rows.push(row![]);
    rows.push(row![
        "",
        format!("Description: {}", textwrap::fill(&vuln.description, 80))
    ]);
    rows.push(row! {});
    rows.push(row![
        "",
        format!("Remediation: {}", textwrap::fill(&vuln.remediation, 80))
    ]);
    rows.push(row! {});

    rows
}

fn check_filter_vuln(filter: &Filter, vuln: &Vulnerability) -> bool {
    let mut include = true;
    if let Some(domains) = &filter.domains {
        if !domains.contains(&RiskDomain::Vulnerabilities) {
            include = false;
        }
    }
    if let Some(level) = &filter.level {
        if vuln.risk_level < *level {
            include = false;
        }
    }
    include
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

impl Summarize for RequestStatusResponse<PackageStatusExtended> {
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
            let rows: Vec<Row> = issue.into();
            for r in rows {
                table_2.add_row(r);
            }
            table_2.add_empty_row();
        }

        let mut vulns_table = Table::new();
        vulns_table.set_format(table_format(3, 0));

        for p in &self.packages {
            for v in &p.vulnerabilities {
                let mut include = true;
                if let Some(ref filter) = filter {
                    include = check_filter_vuln(filter, v);
                }

                if include {
                    for r in
                        vuln_to_rows(v, Some(&p.basic_status.name), Some(&p.basic_status.version))
                    {
                        vulns_table.add_row(r);
                    }
                }
            }
        }

        table_1.printstd();
        table_2.printstd();

        if !vulns_table.is_empty() {
            println!("\n Vulnerabilities:");
            vulns_table.printstd();
        }
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
            let rows: Vec<Row> = issue.into();
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

        let mut vulns_table = Table::new();
        vulns_table.set_format(table_format(3, 0));

        for v in &self.vulnerabilities {
            let mut include = true;
            if let Some(ref filter) = filter {
                if let Some(domains) = &filter.domains {
                    if !domains.contains(&RiskDomain::Vulnerabilities) {
                        include = false;
                    }
                }
                if let Some(level) = &filter.level {
                    if v.risk_level < *level {
                        include = false;
                    }
                }
            }
            if include {
                for r in vuln_to_rows(v, None, None) {
                    vulns_table.add_row(r);
                }
            }
        }

        println!("{}", self.render());

        println!(" Risk Vectors:");
        risks_table.printstd();

        if !issues_table.is_empty() {
            println!("\n Issues:");
            issues_table.printstd();
        }

        if !vulns_table.is_empty() {
            println!("\n Vulnerabilities:");
            vulns_table.printstd();
        }
    }
}

impl<T> Summarize for Vec<T> where T: Renderable {}

impl Summarize for String {}
impl Summarize for PackageStatus {}
impl Summarize for ProjectGetDetailsRequest {}
impl Summarize for AllJobsStatusResponse {}
impl Summarize for CancelRequestResponse {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Vulnerability;

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

        let vuln = Vulnerability {
            base_severity: 0.55,
            cve: vec![],
            risk_level: RiskLevel::Crit,
            title: "Some vuln".to_string(),
            description: "".to_string(),
            remediation: "".to_string(),
        };

        let include = check_filter_vuln(&filter, &vuln);
        assert!(!include);

        let filter_string = "mal";
        let filter = Filter::from_str(filter_string).expect("Failed to parse filter string: {}");
        let include = check_filter_issue(&filter, &issue);
        assert!(!include);
    }
}
