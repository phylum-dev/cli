use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

use crate::restson::{Error, RestPath};

pub type ProjectId = Uuid;
pub type JobId = Uuid;
pub type UserId = Uuid;
pub type Key = Uuid;
pub type PackageId = String;

pub const API_PATH: &str = "api/v0";
pub const PROJ_CONF_FILE: &str = ".phylum_project";

#[serde(rename_all = "lowercase")]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PackageType {
    Npm,
    PyPi,
    Java,
    Ruby,
}

impl FromStr for PackageType {
    type Err = ();

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "npm" => Ok(Self::Npm),
            "pypi" => Ok(Self::PyPi),
            "java" => Ok(Self::Java),
            "ruby" => Ok(Self::Ruby),
            _ => Err(()),
        }
    }
}

impl fmt::Display for PackageType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = format!("{:?}", self);
        write!(f, "{}", s.to_lowercase())
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
pub struct JwtToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApiToken {
    pub active: bool,
    pub key: Key,
    pub user_id: UserId,
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
    pub norecurse: bool,
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
}

impl RestPath<()> for AllJobsStatusResponse {
    fn get_path(_: ()) -> Result<String, Error> {
        Ok(format!("{}/job", API_PATH))
    }
}

/// GET /job/<job_id>
#[derive(Debug, Serialize, Deserialize)]
pub struct StatusRequest {
    job_id: JobId,
}

impl<'a> RestPath<JobId> for RequestStatusResponse {
    fn get_path(job_id: JobId) -> Result<String, Error> {
        Ok(format!("{}/job/{}", API_PATH, job_id))
    }
}

impl<'a> RestPath<JobId> for CancelRequestResponse {
    fn get_path(job_id: JobId) -> Result<String, Error> {
        Ok(format!("{}/job/{}", API_PATH, job_id))
    }
}

/// GET /job/packages/<type>/<name>/<version>
impl<'a> RestPath<PackageDescriptor> for PackageStatus {
    fn get_path(pkg: PackageDescriptor) -> Result<String, Error> {
        let name_escaped = pkg.name.replace("/", "~");
        let endpoint = format!("{}/{}/{}", pkg.r#type, name_escaped, pkg.version);
        Ok(format!("{}/job/packages/{}", API_PATH, endpoint))
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

/// GET /heuristics
#[derive(Debug, Serialize, Deserialize)]
pub struct HeuristicsListResponse {
    pub heuristics: Vec<String>,
}

/// POST /heuristics
#[derive(Debug, Serialize, Deserialize)]
pub struct HeuristicsSubmitRequest {
    pub package: PackageDescriptor,
    pub heuristics_filter: Vec<String>,
    pub include_deps: bool,
}

impl RestPath<()> for HeuristicsSubmitRequest {
    fn get_path(_: ()) -> Result<String, Error> {
        Ok(format!("{}/job/heuristics", API_PATH))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HeuristicsSubmitResponse {
    pub msg: String,
}

impl RestPath<()> for HeuristicsListResponse {
    fn get_path(_: ()) -> Result<String, Error> {
        Ok(format!("{}/job/heuristics", API_PATH))
    }
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
    pub packages: Vec<PackageDescriptor>,
}

/*
#[derive(Debug, Serialize, Deserialize)]
pub struct HeuristicResult {
    score: f64,
    data: Value, // The structure of this data is dependent on the particular heuristic
}*/

#[derive(Debug, Serialize, Deserialize)]
pub struct Package {
    #[serde(flatten)]
    pub package: PackageDescriptor,
    pub license: Option<String>,
    pub package_score: f64,
    pub vulnerabilities: Vec<Value>, // TODO: parse this using a strong type
    pub heuristics: Value,           // TODO: parse this using a strong type
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PackageStatus {
    #[serde(flatten)]
    pub package: Package,
    pub dependencies: Vec<PackageDescriptor>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RequestStatusResponse {
    pub id: JobId,
    pub user_id: UserId,
    pub started_at: u64,   // epoch seconds
    pub last_updated: u64, // epoch seconds
    pub score: f64,
    pub project: Option<ProjectId>,
    pub label: String,
    pub packages: Vec<PackageStatus>,
}

/// DELETE /request/packages/<job_id>
#[derive(Debug, Serialize, Deserialize)]
pub struct CancelRequestResponse {
    pub msg: String,
}
