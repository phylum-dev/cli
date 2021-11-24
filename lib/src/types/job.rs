use serde::{Deserialize, Serialize};

use crate::restson::{Error, RestPath};
use crate::types::API_PATH;

use super::common::*;
use super::package::*;
use super::project::*;

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

impl RestPath<&str> for ProjectGetDetailsRequest {
    fn get_path(pkg_id: &str) -> Result<String, Error> {
        Ok(format!("{}/job/projects/name/{}", API_PATH, pkg_id))
    }
}
