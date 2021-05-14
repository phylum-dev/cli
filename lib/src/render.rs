use crate::types::*;
use ansi_term::Color::{Blue, Green, Red, White};

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

impl Renderable for ApiToken {
    fn render(&self) -> String {
        format!("{:<10} | {:>48}", self.created, self.key)
    }
}

impl Renderable for ProjectGetRequest {
    fn render(&self) -> String {
        let name = format!("{}", White.paint(self.name.clone()));
        format!("{:<38}{}", name, self.id)
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

impl Renderable for ProjectGetDetailsRequest {
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

        let mut ret = String::new();
        ret.push_str(
            format!(
                "{:>15} {:<50} Project ID: {}\n",
                "Project Name:", self.name, self.id
            )
            .as_str(),
        );
        ret.push_str(format!("{:>15} {}\n\n", "Ecosystem:", self.ecosystem).as_str());
        ret.push_str(format!("{:>15} {}\n", "Thresholds:", "Score requirements to PASS or FAIL a run. Runs that have a score below the threshold value will FAIL.").as_str());
        ret.push_str(format!("{:>24}: {}\n", "Project Score", threshold_total).as_str());
        ret.push_str(
            format!(
                "{:>20} {}: {}\n",
                "Malicious Code Risk", "MAL", threshold_malicious
            )
            .as_str(),
        );
        ret.push_str(
            format!(
                "{:>20} {}: {}\n",
                "Vulnerability Risk", "VLN", threshold_vulnerability
            )
            .as_str(),
        );
        ret.push_str(
            format!(
                "{:>20} {}: {}\n",
                "Engineering Risk", "ENG", threshold_engineering
            )
            .as_str(),
        );
        ret.push_str(format!("{:>20} {}: {}\n", "Author Risk", "AUT", threshold_author).as_str());
        ret.push_str(
            format!(
                "{:>20} {}: {}\n\n",
                "License Risk", "LIC", threshold_license
            )
            .as_str(),
        );
        ret.push_str(format!("Last {} jobs from project history\n", self.jobs.len()).as_str());
        ret.push_str(
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

            if !job.pass {
                msg = format!("{}", Red.paint("FAIL"));
                colored_score = format!("{}", Red.paint(&score));
            }

            ret.push_str(
                format!(
                // Differs from the title format slightly. The colored values
                // add control characters, which introduce a base offset of 9
                // zero-width chracters.
                "{:<16}{:<20}{:<41}{:<40}   {}\n",
                colored_score,
                msg,
                job.label,
                job.job_id,
                job.date,
            )
                .as_str(),
            );
        }

        ret.push('\n');
        ret
    }
}

impl Renderable for AllJobsStatusResponse {
    fn render(&self) -> String {
        let mut x = format!(
            "Last {} runs of {} submitted\n\n",
            self.count, self.total_jobs
        );

        for (i, job) in self.jobs.iter().enumerate() {
            let mut state = format!("{}", Green.paint("PASS"));
            let score = format!("{}", (job.score * 100.0) as u32);
            let mut colored_score = format!("{}", Green.paint(&score));
            let project_name = format!("{}", White.bold().paint(job.project.clone()));

            if !job.pass {
                colored_score = format!("{}", Red.paint(&score));
                state = format!("{}", Red.paint("FAIL"));
            }

            let first_line = format!(
                "{}",
                format_args!(
                    "{:<3} {:<5} {} {:<50} {:<30} {:<40} {:>32}\n",
                    (i + 1),
                    colored_score,
                    state,
                    project_name,
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
            x.push_str(first_line.as_str());
            x.push_str(second_line.as_str());
            x.push_str(third_line.as_str());
            x.push_str("\n\n");
        }

        x
    }
}

impl Renderable for JobDescriptor {
    fn render(&self) -> String {
        let mut res = format!(
            "Job id: {}\n====================================\n",
            self.job_id
        );

        for p in &self.packages {
            res.push_str(&p.render());
        }
        res
    }
}

impl Renderable for RequestStatusResponse<PackageStatus> {
    fn render(&self) -> String {
        "TODO".to_string()
    }
}

impl Renderable for RequestStatusResponse<PackageStatusExtended> {
    fn render(&self) -> String {
        "TODO".to_string()
    }
}

impl Renderable for PackageStatus {
    fn render(&self) -> String {
        "TODO".to_string()
    }
}

impl Renderable for PackageStatusExtended {
    fn render(&self) -> String {
        "TODO".to_string()
    }
}

impl Renderable for CancelRequestResponse {
    fn render(&self) -> String {
        format!("Request canceled: {}", self.msg)
    }
}

impl Renderable for PingResponse {
    fn render(&self) -> String {
        format!("Ping response: {}", self.msg)
    }
}
