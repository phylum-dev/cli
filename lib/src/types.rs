use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::str::FromStr;
use uuid::Uuid;

use crate::restson::{Error, RestPath};

pub type JobId = Uuid;
pub type UserId = Uuid;
pub type PackageId = String;

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

#[derive(Debug, Serialize, Deserialize)]
pub struct Token {
    pub access_token: String,
    pub refresh_token: String,
}

/// POST /auth/login
#[derive(Debug, Serialize, Deserialize)]
pub struct AuthRequest {
    pub login: String,
    pub password: String,
}

impl RestPath<()> for AuthRequest {
    fn get_path(_: ()) -> Result<String, Error> {
        Ok(String::from("auth/login"))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResponse {
    #[serde(flatten)]
    pub token: Token,
}

/// PUT /request/packages
#[derive(Debug, Serialize, Deserialize)]
pub struct PackageRequest {
    pub packages: Vec<PackageDescriptor>,
}

impl RestPath<()> for PackageRequest {
    fn get_path(_: ()) -> Result<String, Error> {
        Ok(String::from("request/package"))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PackageSubmissionResponse {
    pub job_id: JobId,
}

/// GET /request/packages/<job_id>
#[derive(Debug, Serialize, Deserialize)]
pub struct StatusRequest {
    job_id: JobId,
}

impl<'a> RestPath<JobId> for RequestStatusResponse {
    fn get_path(job_id: JobId) -> Result<String, Error> {
        Ok(format!("request/package/{}", job_id))
    }
}

impl<'a> RestPath<JobId> for CancelRequestResponse {
    fn get_path(job_id: JobId) -> Result<String, Error> {
        Ok(format!("request/package/{}", job_id))
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
