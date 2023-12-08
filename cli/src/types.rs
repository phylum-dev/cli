use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use phylum_types::types::package::{
    PackageDescriptor, RiskDomain as PTRiskDomain, RiskLevel as PTRiskLevel,
};
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
    pub job_link: Option<String>,
}

/// Policy evaluation results for a dependency.
#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct EvaluatedDependency {
    pub purl: String,
    pub registry: String,
    pub name: String,
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
    pub domain: Option<RiskDomain>,
    pub severity: Option<RiskLevel>,
    pub description: Option<String>,
    pub reason: Option<String>,
}

/// Locksmith token details accessible after creation.
#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct UserToken {
    pub name: String,
    pub creation_time: DateTime<Utc>,
    pub access_time: Option<DateTime<Utc>>,
    pub expiry: Option<DateTime<Utc>>,
}

/// Request body for `/locksmith/v1/revoke`.
#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct RevokeTokenRequest<'a> {
    pub name: &'a str,
}

/// Response body for `/data/packages/submit`.
#[derive(Serialize, Deserialize)]
#[serde(tag = "status", content = "data")]
pub enum PackageSubmitResponse {
    AlreadyProcessed(Package),
    AlreadySubmitted,
    New,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Package {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purl: Option<String>,
    pub id: String,
    pub name: String,
    pub version: String,
    pub registry: String,
    pub published_date: Option<String>,
    pub latest_version: Option<String>,
    pub versions: Vec<ScoredVersion>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub dep_specs: Vec<PackageSpecifier>,
    pub dependencies: Option<Vec<Package>>,
    pub download_count: u32,
    pub risk_scores: RiskScores,
    pub total_risk_score_dynamics: Option<Vec<ScoreDynamicsPoint>>,
    pub issues_details: Vec<Issue>,
    pub issues: Vec<IssuesListItem>,
    pub authors: Vec<Author>,
    pub developer_responsiveness: Option<DeveloperResponsiveness>,
    pub issue_impacts: IssueImpacts,
    pub complete: bool,
    pub release_data: Option<PackageReleaseData>,
    pub repo_url: Option<String>,
    pub maintainers_recently_changed: Option<bool>,
    pub is_abandonware: Option<bool>,
}

#[derive(Serialize, Deserialize)]
pub struct PackageSpecifier {
    #[serde(alias = "type")]
    pub registry: String,
    pub name: String,
    pub version: String,
}

#[derive(Serialize, Deserialize)]
pub struct ScoredVersion {
    pub version: String,
    pub total_risk_score: Option<f32>,
}

/// Package risk scores, broken down by domain.
#[derive(Serialize, Deserialize, Default)]
pub struct RiskScores {
    pub total: f32,
    pub vulnerability: f32,
    #[serde(rename = "malicious_code")]
    #[serde(alias = "malicious")]
    pub malicious: f32,
    pub author: f32,
    pub engineering: f32,
    pub license: f32,
}

/// Package score at a specific point in time.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreDynamicsPoint {
    pub date_time: DateTime<Utc>,
    pub score: f32,
    pub label: String,
}

/// An issue that Phylum has found with a package.
#[derive(Serialize, Deserialize)]
pub struct Issue {
    pub tag: Option<String>,
    pub id: Option<String>,
    pub title: String,
    pub description: String,
    #[serde(alias = "risk_level")]
    pub severity: RiskLevel,
    #[serde(alias = "risk_domain")]
    pub domain: RiskDomain,
    pub details: Option<IssueDetails>,
}

/// Extra information about an issue that depends on the type of issue.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum IssueDetails {
    Vulnerability(VulnDetails),
}

#[derive(Serialize, Deserialize)]
pub struct VulnDetails {
    /// The CVE ids that this vuln is linked to.
    pub cves: Vec<String>,
    /// The CVSS score assigned to this vuln.
    pub cvss: f32,
    /// The CVSS vector string assigned to this vuln.
    pub cvss_vector: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IssuesListItem {
    pub risk_type: RiskType,
    pub score: f32,
    pub impact: RiskLevel,
    pub description: String,
    pub title: String,
    pub tag: Option<String>,
    pub id: Option<String>,
    pub ignored: IgnoredReason,
}

/// The category of risk that an issue poses.
#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Copy, Clone)]
#[serde(rename_all = "camelCase")]
pub enum RiskType {
    TotalRisk,
    Vulnerabilities,
    #[serde(alias = "maliciousRisk")]
    #[serde(rename = "maliciousCodeRisk")]
    MaliciousRisk,
    AuthorsRisk,
    EngineeringRisk,
    LicenseRisk,
}

impl fmt::Display for RiskType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let risk_domain = match self {
            RiskType::MaliciousRisk => "MAL",
            RiskType::Vulnerabilities => "VLN",
            RiskType::EngineeringRisk => "ENG",
            RiskType::AuthorsRisk => "AUT",
            RiskType::LicenseRisk => "LIC",
            RiskType::TotalRisk => "ALL",
        };
        write!(f, "{risk_domain}")
    }
}

impl From<RiskDomain> for RiskType {
    fn from(risk_domain: RiskDomain) -> Self {
        match risk_domain {
            RiskDomain::Malicious => RiskType::MaliciousRisk,
            RiskDomain::Vulnerabilities => RiskType::Vulnerabilities,
            RiskDomain::EngineeringRisk => RiskType::EngineeringRisk,
            RiskDomain::AuthorRisk => RiskType::AuthorsRisk,
            RiskDomain::LicenseRisk => RiskType::LicenseRisk,
        }
    }
}

/// The user-specified reason for an issue to be ignored.
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum IgnoredReason {
    /// It is not ignored.
    False,
    FalsePositive,
    NotRelevant,
    Other,
}

/// One of the authors of a package.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Author {
    pub name: String,
    pub avatar_url: String,
    pub email: String,
    pub profile_url: String,
}

/// Stats about how responsive the maintainers of a package are.
#[derive(Serialize, Deserialize)]
pub struct DeveloperResponsiveness {
    pub open_issue_count: Option<usize>,
    pub total_issue_count: Option<usize>,
    pub open_issue_avg_duration: Option<u32>,
    pub open_pull_request_count: Option<usize>,
    pub total_pull_request_count: Option<usize>,
    pub open_pull_request_avg_duration: Option<u32>,
}

/// The number of issues a package has, broken down by severity.
#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct IssueImpacts {
    pub low: u32,
    pub medium: u32,
    pub high: u32,
    pub critical: u32,
}

/// Information about when package releases have happened.
#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct PackageReleaseData {
    pub first_release_date: String,
    pub last_release_date: String,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Copy, Clone, Debug)]
pub enum RiskDomain {
    /// One or more authors is a possible bad actor or other problems.
    #[serde(rename = "author")]
    AuthorRisk,
    /// Poor engineering practices and other code smells.
    #[serde(rename = "engineering")]
    EngineeringRisk,
    /// Malicious code such as malware or crypto miners.
    #[serde(rename = "malicious_code")]
    #[serde(alias = "malicious")]
    Malicious,
    /// A code vulnerability such as use-after-free or other code smell.
    #[serde(rename = "vulnerability")]
    Vulnerabilities,
    /// License is unknown, incompatible with the project, etc.
    #[serde(rename = "license")]
    LicenseRisk,
}

impl From<PTRiskDomain> for RiskDomain {
    fn from(foreign: PTRiskDomain) -> Self {
        match foreign {
            PTRiskDomain::AuthorRisk => RiskDomain::AuthorRisk,
            PTRiskDomain::EngineeringRisk => RiskDomain::EngineeringRisk,
            PTRiskDomain::Malicious => RiskDomain::Malicious,
            PTRiskDomain::Vulnerabilities => RiskDomain::Vulnerabilities,
            PTRiskDomain::LicenseRisk => RiskDomain::LicenseRisk,
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub enum RiskLevel {
    /// Informational, no action needs to be taken.
    Info,
    /// Minor issues like cosmetic code smells, possibly a problem in great
    /// number or rare circumstances.
    Low,
    /// May be indicative of overall quality issues.
    Medium,
    /// Possibly exploitable behavior in some circumstances.
    High,
    /// Should fix as soon as possible, may be under active exploitation.
    Critical,
}

impl fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let risk_level = format!("{self:?}");
        write!(f, "{}", risk_level.to_lowercase())
    }
}

impl From<PTRiskLevel> for RiskLevel {
    fn from(foreign: PTRiskLevel) -> Self {
        match foreign {
            PTRiskLevel::Info => RiskLevel::Info,
            PTRiskLevel::Low => RiskLevel::Low,
            PTRiskLevel::Medium => RiskLevel::Medium,
            PTRiskLevel::High => RiskLevel::High,
            PTRiskLevel::Critical => RiskLevel::Critical,
        }
    }
}
