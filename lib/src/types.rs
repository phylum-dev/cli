use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::str::FromStr;
use uuid::Uuid;

use crate::restson::{Error, RestPath};

pub type JobId = Uuid;
pub type UserId = Uuid;
pub type Key = Uuid;
pub type PackageId = String;

pub const API_PATH: &str = "api/v0";

#[serde(rename_all = "UPPERCASE")]
#[derive(Debug, Serialize, Deserialize)]
pub enum RequestState {
    New,
    Processing,
    Completed,
    Error,
}

#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[derive(Debug, Serialize, Deserialize)]
pub enum PackageState {
    New,                       // Brand new request, nothing has been processed yet.
    PendingDownload,           // We have issued the download but it has not started yet.
    Downloading,               // We are downloading the package files.
    Processing,                // We are processing the package files.
    PendingExternalProcessing, // Processing of package files is complete; waiting on external processing (e.g. VCS)
    PendingPackageProcessing, // External processing is complete; waiting on processing of package files
    Completed,                // We have completed both downloading and processing.
}

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

#[derive(Debug, Serialize, Deserialize)]
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

/// GET /job
#[derive(Debug, Serialize, Deserialize)]
pub struct AllJobsStatusResponse {
    pub jobs: Vec<RequestStatusResponse>,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackageDescriptor {
    pub name: String,
    pub version: String,
    pub r#type: PackageType,
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
    package: PackageDescriptor,
    last_updated: u64, // epoch seconds
    license: Option<String>,
    risk: f64,
    status: PackageState,
    vulnerabilities: Vec<Value>, // TODO: parse this using a strong type
    heuristics: Value,           // TODO: parse this using a strong type
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PackageStatus {
    #[serde(flatten)]
    package: Package,
    dependencies: Vec<Package>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RequestStatusResponse {
    id: JobId,
    user_id: UserId,
    started_at: u64,   // epoch seconds
    last_updated: u64, // epoch seconds
    status: RequestState,
    packages: Vec<PackageStatus>,
}

/// DELETE /request/packages/<job_id>
#[derive(Debug, Serialize, Deserialize)]
pub struct CancelRequestResponse {
    pub msg: String,
}
