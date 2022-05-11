use ansi_term::Color::{Blue, Green, Red, White, Yellow};
use chrono::{Local, TimeZone};
use phylum_types::types::job::*;
use phylum_types::types::package::*;
use phylum_types::types::project::*;
use prettytable::*;

use crate::print::{self, table_format};

pub trait Renderable {
    fn render(&self) -> String;
}

impl Renderable for () {
    fn render(&self) -> String {
        "".to_string()
    }
}

impl<T> Renderable for Vec<T>
where
    T: Renderable,
{
    fn render(&self) -> String {
        self.iter()
            .map(|t| t.render())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl Renderable for String {
    fn render(&self) -> String {
        self.to_owned()
    }
}

impl Renderable for ProjectSummaryResponse {
    fn render(&self) -> String {
        let name = print::truncate(&self.name, 28);
        format!("{:<37} {}", White.paint(name).to_string(), self.id)
    }
}

impl Renderable for PackageDescriptor {
    fn render(&self) -> String {
        format!("{:<48}{:20}", self.name, self.version)
    }
}

/// Convert the given threshold float value into a string. If no value is
/// returned, i.e. a value of 0, returns a placehold to indicate that this
/// value is unset.
fn threshold_to_str(n: f32) -> String {
    let threshold = (n * 100.0) as u32;

    if threshold == 0 {
        return String::from("Not Set");
    }

    format!("{}", threshold)
}

impl Renderable for ProjectDetailsResponse {
    fn render(&self) -> String {
        let title_score = format!("{}", Blue.paint("Score"));
        let title_passfail = format!("{}", Blue.paint("P/F"));
        let title_label = format!("{}", Blue.paint("Label"));
        let title_job_id = format!("{}", Blue.paint("Job ID"));
        let title_datetime = format!("{}", Blue.paint("Datetime"));

        let threshold_total = threshold_to_str(self.thresholds.total);
        let threshold_malicious = threshold_to_str(self.thresholds.malicious);
        let threshold_vulnerability = threshold_to_str(self.thresholds.vulnerability);
        let threshold_engineering = threshold_to_str(self.thresholds.engineering);
        let threshold_author = threshold_to_str(self.thresholds.author);
        let threshold_license = threshold_to_str(self.thresholds.license);

        let mut renderer = String::new();
        renderer.push_str(
            format!(
                "{:>15} {:<50} Project ID: {}\n",
                "Project Name:", self.name, self.id
            )
            .as_str(),
        );
        renderer.push_str(format!("{:>15} {}\n\n", "Ecosystem:", self.ecosystem).as_str());
        renderer.push_str(format!("{:>15} {}\n", "Thresholds:", "Score requirements to PASS or FAIL a run. Runs that have a score below the threshold value will FAIL.").as_str());
        renderer.push_str(format!("{:>24}: {}\n", "Project Score", threshold_total).as_str());
        renderer.push_str(
            format!(
                "{:>20} {}: {}\n",
                "Malicious Code Risk", "MAL", threshold_malicious
            )
            .as_str(),
        );
        renderer.push_str(
            format!(
                "{:>20} {}: {}\n",
                "Vulnerability Risk", "VLN", threshold_vulnerability
            )
            .as_str(),
        );
        renderer.push_str(
            format!(
                "{:>20} {}: {}\n",
                "Engineering Risk", "ENG", threshold_engineering
            )
            .as_str(),
        );
        renderer
            .push_str(format!("{:>20} {}: {}\n", "Author Risk", "AUT", threshold_author).as_str());
        renderer.push_str(
            format!(
                "{:>20} {}: {}\n\n",
                "License Risk", "LIC", threshold_license
            )
            .as_str(),
        );
        renderer.push_str(format!("Last {} jobs from project history\n", self.jobs.len()).as_str());
        renderer.push_str(
            format!(
                "{:<16}{:<20}{:<50}{:<45}   {}\n",
                title_score, title_passfail, title_label, title_job_id, title_datetime
            )
            .as_str(),
        );

        for job in self.jobs.iter() {
            let score = format!("{}", (job.score * 100.0) as u32);
            let mut colored_score = format!("{}", Green.paint(&score));
            let mut msg = format!("{}", Green.paint("PASS"));

            if job.num_incomplete > 0 {
                msg = format!("{}", Yellow.paint("INCOMPLETE"));
                colored_score = format!("{}", Red.paint(&score));
            } else if !job.pass {
                msg = format!("{}", Red.paint("FAIL"));
                colored_score = format!("{}", Red.paint(&score));
            }

            renderer.push_str(
                format!(
                    // Differs from the title format slightly. The colored values
                    // add control characters, which introduce a base offset of 9
                    // zero-width chracters.
                    "{:<16}{:<20}{:<41}{:<40}   {}\n",
                    colored_score, msg, job.label, job.job_id, job.date,
                )
                .as_str(),
            );
        }

        renderer.push('\n');
        renderer
    }
}

impl Renderable for AllJobsStatusResponse {
    fn render(&self) -> String {
        format!(
            "Last {} runs of {} submitted\n\n{}",
            self.count,
            self.total_jobs,
            self.jobs.render()
        )
    }
}

impl Renderable for Vec<JobDescriptor> {
    fn render(&self) -> String {
        let mut jobs = String::new();
        for (i, job) in self.iter().enumerate() {
            let mut state = Green.paint("PASS").to_string();
            let score = format!("{:>3}", (job.score * 100.0) as u32);
            let mut colored_score = Green.paint(&score).to_string();
            let project_name = print::truncate(&job.project, 39);
            let colored_project_name = White.bold().paint(project_name).to_string();

            if job.num_incomplete > 0 {
                colored_score = format!("{}", Yellow.paint(&score));
                state = format!("{}", Yellow.paint("INCOMPLETE"));
            } else if !job.pass {
                colored_score = format!("{}", Red.paint(&score));
                state = format!("{}", Red.paint("FAIL"));
            }

            let first_line = format!(
                "{}",
                format_args!(
                    "{:<3} {} {} {:<50} {:<30} {:<40} {:>32}\n",
                    (i + 1),
                    colored_score,
                    state,
                    colored_project_name,
                    job.label,
                    job.job_id,
                    job.date
                )
            );
            let second_line = format!("             {}\n", job.msg);
            let third_line = format!(
                "             {}{:>62}{:>29} dependencies",
                job.ecosystem, "Crit:-/High:-/Med:-/Low:-", job.num_dependencies
            );
            jobs.push_str(first_line.as_str());
            jobs.push_str(second_line.as_str());
            jobs.push_str(third_line.as_str());
            jobs.push_str("\n\n");
        }
        jobs.trim_end().into()
    }
}

impl Renderable for JobStatusResponse<PackageStatus> {
    fn render(&self) -> String {
        "TODO".to_string()
    }
}

impl Renderable for JobStatusResponse<PackageStatusExtended> {
    fn render(&self) -> String {
        "TODO".to_string()
    }
}

impl Renderable for PackageStatus {
    fn render(&self) -> String {
        let mut table = table!(
            ["Package Name:", self.name, "Package Version:", self.version],
            [
                "License:",
                self.license.as_ref().unwrap_or(&"Unknown".to_string()),
                "Last updated:",
                self.last_updated
            ],
            [
                "Num Deps:",
                self.num_dependencies,
                "Num Vulns:",
                self.num_vulnerabilities
            ]
        );

        table.set_format(table_format(0, 0));
        table.to_string()
    }
}

impl Renderable for PackageType {
    fn render(&self) -> String {
        let label = match self {
            PackageType::Npm => "NPM",
            PackageType::RubyGems => "RubyGems",
            PackageType::PyPi => "PyPI",
            PackageType::Maven => "Maven",
            PackageType::Nuget => "NuGet",
        };
        label.to_owned()
    }
}

impl Renderable for PackageStatusExtended {
    fn render(&self) -> String {
        let time = Local
            .timestamp_opt((self.basic_status.last_updated / 1000) as i64, 0)
            .single()
            .map(|time| time.format("%+").to_string())
            .unwrap_or_else(|| String::from("UNKNOWN"));

        let mut overview_table = table!(
            ["Package Name:", rB -> self.basic_status.name, "Package Version:", r -> self.basic_status.version],
            ["License:", r -> self.basic_status.license.as_ref().unwrap_or(&"Unknown".to_string()), "Last updated:", r -> time],
            ["Num Deps:", r -> self.basic_status.num_dependencies, "Num Vulns:", r -> self.basic_status.num_vulnerabilities],
            ["Ecosystem:", r -> self.package_type.render()]
        );
        overview_table.set_format(table_format(0, 3));
        overview_table.to_string()
    }
}

impl Renderable for CancelJobResponse {
    fn render(&self) -> String {
        format!("Request canceled: {}", self.msg)
    }
}

impl Renderable for ProjectThresholds {
    fn render(&self) -> String {
        let normalize = |t: f32| (t * 100.0).round() as u32;
        let mut table = table!(
            [r => "Thresholds:"],
            [r => "Project Score:", normalize(self.total)],
            [r => "Malicious Code Risk MAL:", normalize(self.malicious)],
            [r => "Vulnerability Risk VLN:", normalize(self.vulnerability)],
            [r => "Engineering Risk ENG:", normalize(self.engineering)],
            [r => "Author Risk AUT:", normalize(self.author)],
            [r => "License Risk LIC:", normalize(self.license)]
        );
        table.set_format(table_format(0, 0));
        table.to_string()
    }
}
