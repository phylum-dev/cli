use crate::types::*;

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

impl Renderable for PackageDescriptor {
    fn render(&self) -> String {
        format!("{:<48}{:20}", self.name, self.version)
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
