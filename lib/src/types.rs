use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::restson::{Error, RestPath};

pub type ProjectId = Uuid;
pub type JobId = Uuid;
pub type UserId = Uuid;
pub type Key = Uuid;
pub type PackageId = String;

pub const API_PATH: &str = "api/v0";
pub const PROJ_CONF_FILE: &str = ".phylum_project";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageType {
    Npm,
    Python,
    Java,
    Ruby,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectThresholds {
    pub author: f32,
    pub engineering: f32,
    pub license: f32,
    pub malicious: f32,
    pub total: f32,
    pub vulnerability: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    None,
    Warn,
    Break,
}

impl FromStr for PackageType {
    type Err = ();

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input.to_lowercase().as_str() {
            "npm" => Ok(Self::Npm),
            "python" => Ok(Self::Python),
            "java" => Ok(Self::Java),
            "ruby" => Ok(Self::Ruby),
            _ => Err(()),
        }
    }
}

impl fmt::Display for PackageType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let package_type = format!("{:?}", self);
        write!(f, "{}", package_type.to_lowercase())
    }
}

impl PackageType {
    pub fn language(&self) -> &str {
        match self {
            PackageType::Npm => "Javascript",
            PackageType::Ruby => "Ruby",
            PackageType::Python => "Python",
            PackageType::Java => "Java",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Role {
    #[serde(rename = "a")]
    Administrator,
    #[serde(rename = "o")]
    Observer,
    #[serde(rename = "u")]
    User,
}

impl FromStr for Role {
    type Err = ();

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "a" => Ok(Self::Administrator),
            "o" => Ok(Self::Observer),
            "u" => Ok(Self::User),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Complete,
    Incomplete,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JwtToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApiToken {
    pub active: bool,
    pub key: Key,
    pub user_id: UserId,
    pub name: Option<String>,
    pub created: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Projecct {
    pub score: u32,
    pub passing: bool,
    pub name: String,
    pub id: ProjectId,
    pub last_updated: u64,
}

/// PUT /authenticate/register
#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub confirm_password: String,
    pub first_name: String,
    pub last_name: String,
}

impl RestPath<()> for RegisterRequest {
    fn get_path(_: ()) -> Result<String, Error> {
        Ok(format!("{}/authenticate/register", API_PATH))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterResponse {
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub role: Role,
    pub user_id: UserId,
}

/// POST /authenticate/login
#[derive(Debug, Serialize, Deserialize)]
pub struct AuthRequest {
    pub email: String,
    pub password: String,
}

impl RestPath<()> for AuthRequest {
    fn get_path(_: ()) -> Result<String, Error> {
        Ok(format!("{}/authenticate/login", API_PATH))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResponse {
    #[serde(flatten)]
    pub token: JwtToken,
}

/// POST /authenticate/refresh
#[derive(Debug, Serialize, Deserialize)]
pub struct RefreshRequest {}

impl RestPath<()> for RefreshRequest {
    fn get_path(_: ()) -> Result<String, Error> {
        Ok(format!("{}/authenticate/refresh", API_PATH))
    }
}

/// PUT /authenticate/key
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiCreateTokenRequest {}

impl RestPath<()> for ApiCreateTokenRequest {
    fn get_path(_: ()) -> Result<String, Error> {
        Ok(format!("{}/authenticate/key", API_PATH))
    }
}

/// DELETE /authenticate/key/<api_key>
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiDeleteTokenRequest {}

impl<'a> RestPath<Key> for ApiDeleteTokenRequest {
    fn get_path(key: Key) -> Result<String, Error> {
        Ok(format!("{}/authenticate/key/{}", API_PATH, key))
    }
}

/// GET /authenticate/key
#[derive(Debug, Serialize, Deserialize)]
pub struct GetApiTokensResponse {
    pub keys: Vec<ApiToken>,
}

impl RestPath<()> for GetApiTokensResponse {
    fn get_path(_: ()) -> Result<String, Error> {
        Ok(format!("{}/authenticate/key", API_PATH))
    }
}

/// PUT /job
#[derive(Debug, Serialize, Deserialize)]
pub struct PackageRequest {
    pub r#type: PackageType,
    pub packages: Vec<PackageDescriptor>,
    pub is_user: bool,
    pub project: ProjectId,
    pub label: String,
}

impl RestPath<()> for PackageRequest {
    fn get_path(_: ()) -> Result<String, Error> {
        Ok(format!("{}/job", API_PATH))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PackageSubmissionResponse {
    pub job_id: JobId,
}

/// GET /job/auth_status
#[derive(Debug, Serialize, Deserialize)]
pub struct AuthStatusResponse {
    pub authenticated: bool,
}

impl RestPath<()> for AuthStatusResponse {
    fn get_path(_: ()) -> Result<String, Error> {
        Ok(format!("{}/job/auth_status", API_PATH))
    }
}

/// GET /job/heartbeat
#[derive(Debug, Serialize, Deserialize)]
pub struct PingResponse {
    pub msg: String,
}

impl RestPath<()> for PingResponse {
    fn get_path(_: ()) -> Result<String, Error> {
        Ok(format!("{}/job/heartbeat", API_PATH))
    }
}

/// GET /job
#[derive(Debug, Serialize, Deserialize)]
pub struct AllJobsStatusResponse {
    pub jobs: Vec<JobDescriptor>,
    pub total_jobs: u32,
    pub count: u32,
}

impl RestPath<u32> for AllJobsStatusResponse {
    fn get_path(limit: u32) -> Result<String, Error> {
        Ok(format!("{}/job/?limit={}&verbose=1", API_PATH, limit))
    }
}

/// GET /job/<job_id>
#[derive(Debug, Serialize, Deserialize)]
pub struct StatusRequest {
    job_id: JobId,
}

impl<'a> RestPath<JobId> for RequestStatusResponse<PackageStatus> {
    fn get_path(job_id: JobId) -> Result<String, Error> {
        Ok(format!("{}/job/{}", API_PATH, job_id))
    }
}

impl<'a> RestPath<JobId> for RequestStatusResponse<PackageStatusExtended> {
    fn get_path(job_id: JobId) -> Result<String, Error> {
        Ok(format!("{}/job/{}?verbose=True", API_PATH, job_id))
    }
}

impl<'a> RestPath<JobId> for CancelRequestResponse {
    fn get_path(job_id: JobId) -> Result<String, Error> {
        Ok(format!("{}/job/{}", API_PATH, job_id))
    }
}

/// GET /job/packages/<type>/<name>/<version>
impl<'a> RestPath<PackageDescriptor> for PackageStatusExtended {
    fn get_path(pkg: PackageDescriptor) -> Result<String, Error> {
        let name_escaped = pkg.name.replace("/", "~");
        let endpoint = format!("{}/{}/{}", pkg.r#type, name_escaped, pkg.version);
        Ok(format!("{}/job/packages/{}", API_PATH, endpoint))
    }
}

/// GET /projects/overview
#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectGetRequest {
    pub name: String,
    pub id: String,
    pub updated_at: String,
    // TODO: Need to update request manager to include thresholds with this
    //       response.
    //pub thresholds: ProjectThresholds,
}

impl RestPath<()> for Vec<ProjectGetRequest> {
    fn get_path(_: ()) -> Result<String, Error> {
        Ok(format!("{}/job/projects/overview", API_PATH))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectGetResponse {
    pub id: ProjectId,
}

/// GET /projects/<project-id>
#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectGetDetailsRequest {
    pub name: String,
    pub id: String,
    pub ecosystem: String,
    pub thresholds: ProjectThresholds,
    pub jobs: Vec<JobDescriptor>,
}

impl RestPath<&str> for ProjectGetDetailsRequest {
    fn get_path(pkg_id: &str) -> Result<String, Error> {
        Ok(format!("{}/job/projects/name/{}", API_PATH, pkg_id))
    }
}

/// PUT /settings/current-user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Threshold {
    pub action: String,
    pub active: bool,
    pub threshold: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProject {
    pub thresholds: HashMap<String, Threshold>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Setting {
    DefaultLabel(HashMap<String, String>),
    Project(UserProject),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserSettings {
    pub version: u32,
    pub projects: HashMap<String, Setting>,
}

impl UserSettings {
    /// Sets the threshold for the given risk domain.
    pub fn set_threshold(
        &mut self,
        project_id: String,
        name: String,
        threshold: i32,
        action: String,
    ) {
        log::debug!("Retrieving user settings for project: {}", project_id);
        let mut thresholds = self
            .projects
            .get(project_id.as_str())
            .map(|s| s.to_owned())
            .unwrap_or_else(|| {
                Setting::Project(UserProject {
                    thresholds: HashMap::new(),
                })
            });

        if let Setting::Project(ref mut t) = thresholds {
            t.thresholds.insert(
                name,
                Threshold {
                    action,
                    active: (threshold > 0),
                    threshold: (threshold as f32) / 100.0,
                },
            );
        }

        self.projects.insert(project_id, thresholds);
    }
}

impl RestPath<()> for UserSettings {
    fn get_path(_: ()) -> Result<String, Error> {
        Ok(format!("{}/settings/current-user", API_PATH))
    }
}

/// PUT /projects
#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectCreateRequest {
    pub name: String,
}

impl RestPath<()> for ProjectCreateRequest {
    fn get_path(_: ()) -> Result<String, Error> {
        Ok(format!("{}/job/projects", API_PATH))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectCreateResponse {
    pub id: ProjectId,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackageDescriptor {
    pub name: String,
    pub version: String,
    pub r#type: PackageType,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JobDescriptor {
    pub job_id: JobId,
    pub project: String,
    pub label: String,
    pub num_dependencies: u32,
    pub score: f64,
    pub packages: Vec<PackageDescriptor>,
    pub pass: bool,
    pub msg: String,
    pub date: String,
    pub ecosystem: String,
    #[serde(default)]
    pub num_incomplete: u32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PackageStatus {
    pub name: String,
    pub version: String,
    pub status: Status,
    pub last_updated: u64,
    pub license: Option<String>,
    pub package_score: Option<f64>,
    pub num_dependencies: u32,
    pub num_vulnerabilities: u32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PackageStatusExtended {
    #[serde(flatten)]
    pub basic_status: PackageStatus,
    pub r#type: PackageType,
    #[serde(rename = "riskVectors")]
    pub risk_vectors: HashMap<String, f64>,
    pub dependencies: Vec<PackageDescriptor>,
    pub vulnerabilities: Vec<Vulnerability>,
    pub heuristics: HashMap<String, HeuristicResult>,
    pub issues: Vec<Issue>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, PartialOrd, Ord, Serialize)]
pub enum RiskLevel {
    #[serde(rename = "info")]
    Info,
    #[serde(rename = "low")]
    Low,
    #[serde(rename = "medium")]
    Med,
    #[serde(rename = "high")]
    High,
    #[serde(rename = "critical")]
    Crit,
}

impl fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let risk_level = format!("{:?}", self);
        write!(f, "{}", risk_level.to_lowercase())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum RiskDomain {
    MaliciousCode,
    Vulnerabilities,
    EngineeringRisk,
    AuthorRisk,
    LicenseRisk,
}

impl fmt::Display for RiskDomain {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let risk_domain = match self {
            RiskDomain::MaliciousCode => "MAL",
            RiskDomain::Vulnerabilities => "VLN",
            RiskDomain::EngineeringRisk => "ENG",
            RiskDomain::AuthorRisk => "AUT",
            RiskDomain::LicenseRisk => "LIC",
        };
        write!(f, "{}", risk_domain)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Issue {
    pub title: String,
    pub description: String,
    pub risk_level: RiskLevel,
    pub risk_domain: RiskDomain,
    pub pkg_name: String,
    pub pkg_version: String,
    pub score: f64,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct HeuristicResult {
    pub domain: RiskDomain,
    pub score: f64,
    pub risk_level: RiskLevel,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct Vulnerability {
    pub cve: Vec<String>,
    pub base_severity: f32,
    pub risk_level: RiskLevel,
    pub title: String,
    pub description: String,
    pub remediation: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RequestStatusResponse<T> {
    pub job_id: JobId,
    pub ecosystem: String,
    pub user_id: UserId,
    pub user_email: String,
    pub created_at: i64, // epoch seconds
    pub status: Status,
    pub score: f64,
    pub pass: bool,
    pub msg: String,
    pub action: Action,
    #[serde(default)]
    pub num_incomplete: u32,
    pub last_updated: u64,
    pub project: String, // project id
    pub project_name: String,
    pub label: Option<String>,
    pub thresholds: ProjectThresholds,
    pub packages: Vec<T>,
}

/// DELETE /request/packages/<job_id>
#[derive(Debug, Serialize, Deserialize)]
pub struct CancelRequestResponse {
    pub msg: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GithubRelease {
    pub name: String,
    pub assets: Vec<GithubReleaseAsset>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GithubReleaseAsset {
    pub browser_download_url: String,
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_level_ordering() {
        assert!(
            RiskLevel::Info < RiskLevel::Low
                && RiskLevel::Low < RiskLevel::Med
                && RiskLevel::Med < RiskLevel::High
                && RiskLevel::High < RiskLevel::Crit,
            "Ordering of risk levels is invalid"
        );
    }
}
