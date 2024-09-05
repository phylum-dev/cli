use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use chrono::{DateTime, Utc};
use phylum_lockfile::ParsedLockfile;
use phylum_types::types::common::{JobId, ProjectId};
use phylum_types::types::package::{
    PackageDescriptor, PackageDescriptorAndLockfile, PackageType, RiskDomain as PTRiskDomain,
    RiskLevel as PTRiskLevel,
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
    AlreadyProcessed(Box<Package>),
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
    pub issues: Vec<Issue>,
    pub authors: Vec<Author>,
    pub developer_responsiveness: Option<DeveloperResponsiveness>,
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
#[derive(Clone, Serialize, Deserialize)]
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
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum IssueDetails {
    Vulnerability(VulnDetails),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct VulnDetails {
    /// The CVE ids that this vuln is linked to.
    pub cves: Vec<String>,
    /// The CVSS score assigned to this vuln.
    pub cvss: f32,
    /// The CVSS vector string assigned to this vuln.
    pub cvss_vector: String,
}

/// The user-specified reason for an issue to be ignored.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, Serialize, Deserialize)]
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

/// Information about when package releases have happened.
#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct PackageReleaseData {
    pub first_release_date: String,
    pub last_release_date: String,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Copy, Clone, Debug, Hash)]
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

impl fmt::Display for RiskDomain {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let risk_domain = match self {
            RiskDomain::Malicious => "MAL",
            RiskDomain::Vulnerabilities => "VLN",
            RiskDomain::EngineeringRisk => "ENG",
            RiskDomain::AuthorRisk => "AUT",
            RiskDomain::LicenseRisk => "LIC",
        };
        write!(f, "{risk_domain}")
    }
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

/// Package descriptor formats accepted by analysis endpoint.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AnalysisPackageDescriptor {
    PackageDescriptor(PackageDescriptorAndLockfile),
    Purl(PurlWithOrigin),
}

impl AnalysisPackageDescriptor {
    pub fn descriptors_from_lockfile(
        parsed_lockfile: ParsedLockfile,
    ) -> Vec<AnalysisPackageDescriptor> {
        parsed_lockfile
            .packages
            .iter()
            .map(|package_descriptor| {
                AnalysisPackageDescriptor::PackageDescriptor(PackageDescriptorAndLockfile {
                    package_descriptor: package_descriptor.clone(),
                    lockfile: Some(parsed_lockfile.path.clone()),
                })
            })
            .collect()
    }
}

/// Submit Package for analysis
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, Serialize, Deserialize)]
pub struct SubmitPackageRequest {
    /// The subpackage dependencies of this package
    pub packages: Vec<AnalysisPackageDescriptor>,
    /// Was this submitted by a user interactively and not a CI?
    pub is_user: bool,
    /// The id of the project this top level package should be associated with
    pub project: ProjectId,
    /// A label for this package. Often it's the branch.
    pub label: String,
    /// The group that owns the project, if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_name: Option<String>,
}

/// Package URL with attached dependency file origin.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, Serialize, Deserialize)]
pub struct PurlWithOrigin {
    purl: String,
    // NOTE: This is named `lockfile` with an `origin` alias because the API does
    // not support the `origin` name. So we force conversion to `lockfile` while
    // allowing a more proper name through the alias.
    #[serde(alias = "origin")]
    #[serde(skip_serializing_if = "Option::is_none")]
    lockfile: Option<String>,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, Serialize, Deserialize)]
pub struct UserGroup {
    pub created_at: DateTime<Utc>,
    pub last_modified: DateTime<Utc>,
    pub group_id: Option<String>,
    pub group_name: String,
    pub role: GroupRole,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GroupRole {
    Member,
    Admin,
}
pub type OrgRole = GroupRole;

impl Display for GroupRole {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Member => write!(f, "Member"),
            Self::Admin => write!(f, "Admin"),
        }
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, Serialize, Deserialize)]
pub struct ListUserGroupsResponse {
    pub groups: Vec<UserGroup>,
}

/// Response from Phylum's GET /organizations endpoint.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, Serialize, Deserialize)]
pub struct OrgsResponse {
    pub organizations: Vec<Org>,
}

/// Organization returned by Phylum's GET /organizations endpoint.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, Serialize, Deserialize)]
pub struct Org {
    pub name: String,
}

/// Response from Phylum's GET /organizations/<org>/members endpoint.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, Serialize, Deserialize)]
pub struct OrgMembersResponse {
    pub members: Vec<OrgMember>,
}

/// Member returned by Phylum's GET /organizations/<org>/members endpoint.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, Serialize, Deserialize)]
pub struct OrgMember {
    pub email: String,
    pub role: OrgRole,
}

/// Request body for Phylum's POST /organizations/<org>/members endpoint.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, Serialize, Deserialize)]
pub struct AddOrgUserRequest {
    pub email: String,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, Serialize, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_label: Option<String>,
}
pub type UpdateProjectRequest = CreateProjectRequest;

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, Serialize, Deserialize)]
pub struct ProjectListEntry {
    pub id: ProjectId,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub name: String,
    pub ecosystems: Vec<PackageType>,
    pub group_name: Option<String>,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, Serialize, Deserialize)]
pub struct Paginated<T> {
    /// The curent page of values.
    pub values: Vec<T>,
    /// Indication of whether the current query has more values past the last
    /// element in `values`.
    pub has_more: bool,
}

/// Response to the GET /data/project/<project_id> endpoint.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetProjectResponse {
    pub id: ProjectId,
    pub name: String,
    pub registries: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub latest_job_created_at: Option<DateTime<Utc>>,
    pub latest_job_id: Option<JobId>,
    pub label: Option<String>,
    pub default_label: Option<String>,
    pub repository_url: Option<String>,
}

/// Response body for Phylum's GET /organizations/<org>/groups endpoint.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, Serialize, Deserialize)]
pub struct OrgGroupsResponse {
    pub groups: Vec<OrgGroup>,
}

/// Group returned by Phylum's GET /organizations/<org>/groups endpoint.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, Serialize, Deserialize)]
pub struct OrgGroup {
    pub name: String,
}
