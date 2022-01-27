use phylum_types::types::common::*;
use phylum_types::types::job::*;
use phylum_types::types::package::*;
use phylum_types::types::project::*;

use super::common::API_PATH;
use crate::restson::{Error, RestPath};
use crate::types::AuthStatusResponse;
use crate::types::PingResponse;

/// PUT /job
impl RestPath<()> for SubmitPackageRequest {
    fn get_path(_: ()) -> Result<String, Error> {
        Ok(format!("{}/job", API_PATH))
    }
}

/// GET /job/auth_status
impl RestPath<()> for AuthStatusResponse {
    fn get_path(_: ()) -> Result<String, Error> {
        Ok(format!("{}/job/auth_status", API_PATH))
    }
}

/// GET /job/heartbeat
// TODO Deprecate
impl RestPath<()> for PingResponse {
    fn get_path(_: ()) -> Result<String, Error> {
        Ok(format!("{}/job/heartbeat", API_PATH))
    }
}

/// GET /job/
impl RestPath<u32> for AllJobsStatusResponse {
    fn get_path(limit: u32) -> Result<String, Error> {
        Ok(format!("{}/job/?limit={}&verbose=1", API_PATH, limit))
    }
}

/// Get /job/<job_id> summary
impl<'a> RestPath<JobId> for JobStatusResponse<PackageStatus> {
    fn get_path(job_id: JobId) -> Result<String, Error> {
        Ok(format!("{}/job/{}", API_PATH, job_id))
    }
}

/// Get /job/<job_id> verbose
impl<'a> RestPath<JobId> for JobStatusResponse<PackageStatusExtended> {
    fn get_path(job_id: JobId) -> Result<String, Error> {
        Ok(format!("{}/job/{}?verbose=True", API_PATH, job_id))
    }
}

/// DELETE /request/packages/<job_id>
impl<'a> RestPath<JobId> for CancelJobResponse {
    fn get_path(job_id: JobId) -> Result<String, Error> {
        Ok(format!("{}/job/{}", API_PATH, job_id))
    }
}

/// GET /job/packages/<type>/<name>/<version>
impl<'a> RestPath<PackageDescriptor> for PackageStatusExtended {
    fn get_path(pkg: PackageDescriptor) -> Result<String, Error> {
        let name_escaped = pkg.name.replace("/", "~");
        let endpoint = format!("{}/{}/{}", pkg.package_type, name_escaped, pkg.version);
        Ok(format!("{}/job/packages/{}", API_PATH, endpoint))
    }
}

impl RestPath<&str> for ProjectDetailsResponse {
    fn get_path(pkg_id: &str) -> Result<String, Error> {
        Ok(format!("{}/job/projects/name/{}", API_PATH, pkg_id))
    }
}
