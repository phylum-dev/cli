use std::fmt;
use std::str::FromStr;

use chrono::NaiveDateTime;
use fake::{Fake, Faker};
use prettytable::{Row, Table};

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

            let bucket_id = ((y - min) / step) as usize;
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
        buckets
    }
}

impl fmt::Display for Histogram {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let scale = 32.0 / *self.values.iter().max().unwrap_or(&1) as f32;

        let output =
            self.values
                .iter()
                .zip(self.buckets().iter())
                .fold("".to_string(), |acc, x| {
                    vec![
                        acc,
                        format!(
                            "{:>4} - {:<4} [{:>5}] {}",
                            (100.0 * x.1 .0).round() as u32,
                            (100.0 * x.1 .1).round() as u32,
                            x.0,
                            "█".repeat((*x.0 as f32 * scale) as usize)
                        ),
                    ]
                    .join("\n")
                });

        write!(f, "{:^10} {:>8}", "Score", "Count")?;
        write!(f, "{}", output)
    }
}

pub trait Summarize: Renderable {
    fn summarize(&self) {
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
    let ecosystem = PackageType::from_str(&resp.ecosystem);
    let language = ecosystem
        .map(|e| e.language().to_string())
        .unwrap_or_default();

    let dt = NaiveDateTime::from_timestamp(resp.created_at / 1000, 0);

    let details = [
        (
            "Project",
            resp.project.to_string(),
            "Label",
            resp.label.as_ref().unwrap_or(&"".to_string()).to_owned(),
        ),
        (
            "Proj Score",
            (100.0 * resp.score).round().to_string(),
            "Date",
            format!("{} UTC", dt),
        ),
        (
            "Num Deps",
            resp.packages.len().to_string(),
            "Job ID",
            resp.job_id.to_string(),
        ),
        ("Type", resp.ecosystem.to_uppercase(), "Language", language),
        (
            "User ID",
            resp.user_email.to_string(),
            "View in Phylum UI",
            format!("https://app.phylum.io/{}", resp.job_id),
        ),
    ];
    let summary = details.iter().fold("".to_string(), |acc, x| {
        vec![
            acc,
            format!("{:>16}: {:<36} {:>24}: {:<36}", x.0, x.1, x.2, x.3),
        ]
        .join("\n")
    });

    let scores: Vec<f64> = resp.packages.iter().map(|p| p.score()).collect();

    let hist = Histogram::new(scores.as_slice(), 0.0, 1.0, 10);

    let mut t = table!([hist.to_string(), resp.thresholds.render()]);
    t.set_format(table_format(1, 36));

    let mut ret = Table::new();
    ret.add_row(row![summary]);
    ret.add_row(row![t]);
    ret.set_format(table_format(0, 0));
    ret
}

impl Summarize for RequestStatusResponse<PackageStatus> {
    fn summarize(&self) {
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

    rows.push(row![b -> format!("* {} (base severity: {})", cve_s, vuln.base_severity), r -> &pkg_descriptor]);
    rows.push(row![]);
    rows.push(row![format!(
        "Description: {}",
        textwrap::fill(&vuln.description, 80)
    )]);
    rows.push(row! {});
    rows.push(row![format!(
        "Remediation: {}",
        textwrap::fill(&vuln.remediation, 80)
    )]);
    rows.push(row! {});

    rows
}

impl Summarize for RequestStatusResponse<PackageStatusExtended> {
    fn summarize(&self) {
        let t1: Table = response_to_table(self);

        let mut t2 = Table::new();
        t2.set_format(table_format(3, 1));

        let issues: Vec<Issue> = self
            .packages
            .iter()
            .map(|p| {
                p.heuristics.iter().map(move |(k, v)| {
                    Issue {
                        name: k.to_string(),
                        pkg_name: p.basic_status.name.to_string(),
                        pkg_version: p.basic_status.version.to_string(),
                        risk_level: Faker.fake(), // TODO: update when the api supports this
                        risk_domain: v.domain.to_owned(),
                        score: (v.score * 100.0).round(),
                        description: v.description.to_string(),
                    }
                })
            })
            .flatten()
            .collect();

        for i in issues {
            let rows: Vec<Row> = i.into();
            for r in rows {
                t2.add_row(r);
            }
            t2.add_empty_row();
        }

        let mut vulns_table = Table::new();
        vulns_table.set_format(table_format(3, 0));

        for p in &self.packages {
            for v in &p.vulnerabilities {
                for r in vuln_to_rows(v, Some(&p.basic_status.name), Some(&p.basic_status.version))
                {
                    vulns_table.add_row(r);
                }
            }
        }

        t1.printstd();
        t2.printstd();

        if !vulns_table.is_empty() {
            println!("\n Vulnerabilities:");
            vulns_table.printstd();
        }
    }
}

impl Summarize for PackageStatusExtended {
    fn summarize(&self) {
        let mut issues_table = Table::new();
        issues_table.set_format(table_format(3, 0));

        let issues: Vec<Issue> = self
            .heuristics
            .iter()
            .map(move |(k, v)| {
                Issue {
                    name: k.to_string(),
                    pkg_name: self.basic_status.name.to_string(),
                    pkg_version: self.basic_status.version.to_string(),
                    risk_level: Faker.fake(), // TODO: update when the api supports this
                    risk_domain: v.domain.to_owned(),
                    score: (v.score * 100.0).round(),
                    description: v.description.to_string(),
                }
            })
            .collect();

        for i in issues {
            let rows: Vec<Row> = i.into();
            for mut r in rows {
                r.remove_cell(2);
                issues_table.add_row(r);
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
            for r in vuln_to_rows(v, None, None) {
                vulns_table.add_row(r);
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
