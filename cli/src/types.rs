use std::str::FromStr;

use chrono::{DateTime, Utc};
use phylum_types::types::package::PackageDescriptor;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct PingResponse {
    pub response: String,
}

// TODO Deprecate
#[derive(Debug, Serialize, Deserialize)]
pub struct AuthStatusResponse {
    pub authenticated: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Admin,
    User,
    Observer,
}

impl FromStr for Role {
    type Err = ();

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "administrator" => Ok(Self::Admin),
            "observer" => Ok(Self::Observer),
            "user" => Ok(Self::User),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct GithubRelease {
    pub tag_name: String,
    pub assets: Vec<GithubReleaseAsset>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GithubReleaseAsset {
    pub browser_download_url: String,
    pub name: String,
}

/// History job entry.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HistoryJob {
    pub id: String,
    pub created: DateTime<Utc>,
    pub label: Option<String>,
}

/// Request body for `/data/jobs/{job_id}/policy/evaluate`.
#[derive(Serialize, Debug)]
pub struct PolicyEvaluationRequest {
    pub ignored_packages: Vec<PackageDescriptor>,
}

/// Response body for `/data/jobs/{job_id}/policy/evaluate`.
#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct PolicyEvaluationResponse {
    pub is_failure: bool,
    pub incomplete_count: u32,
    pub output: String,
    pub report: String,
}

/// Response body for `/data/jobs/{job_id}/policy/evaluate/raw`.
#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct PolicyEvaluationResponseRaw {
    pub is_failure: bool,
    pub incomplete_packages_count: u32,
    pub help: String,
    pub dependencies: Vec<EvaluatedDependency>,
    pub job_link: String,
}

/// Policy evaluation results for a dependency.
#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct EvaluatedDependency {
    pub purl: String,
    pub registry: String,
    pub version: String,
    pub rejections: Vec<PolicyRejection>,
}

/// Policy rejection cause.
#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct PolicyRejection {
    pub title: String,
    pub source: RejectionSource,
    pub suppressed: bool,
}

/// Policy rejection source.
#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct RejectionSource {
    #[serde(rename = "type")]
    pub source_type: String,
    pub tag: Option<String>,
    pub domain: Option<String>,
    pub severity: Option<String>,
    pub description: Option<String>,
    pub reason: Option<String>,
}

#[cfg(test)]
mod tests {
    use phylum_types::types::package::RiskLevel;

    #[test]
    fn test_risk_level_ordering() {
        assert!(
            RiskLevel::Info < RiskLevel::Low
                && RiskLevel::Low < RiskLevel::Medium
                && RiskLevel::Medium < RiskLevel::High
                && RiskLevel::High < RiskLevel::Critical,
            "Ordering of risk levels is invalid"
        );
    }
}
