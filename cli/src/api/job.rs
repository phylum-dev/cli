use super::common::API_PATH;
use super::{JobId, PackageDescriptor};

/// PUT /job
pub(crate) fn put_submit_package(api_uri: &str) -> String {
    format!("{api_uri}/{API_PATH}/job")
}

/// GET /job/auth_status
pub(crate) fn get_auth_status(api_uri: &str) -> String {
    format!("{api_uri}/{API_PATH}/job/auth_status")
}

/// GET /job/heartbeat
// TODO Deprecate
pub(crate) fn get_ping(api_uri: &str) -> String {
    format!("{api_uri}/{API_PATH}/job/heartbeat")
}

/// GET /job/
pub(crate) fn get_all_jobs_status(api_uri: &str, limit: u32) -> String {
    format!("{api_uri}/{API_PATH}/job/?limit={limit}&verbose=1")
}

/// Get /job/<job_id>
pub(crate) fn get_job_status(api_uri: &str, job_id: &JobId, verbose: bool) -> String {
    if verbose {
        format!("{api_uri}/{API_PATH}/job/{job_id}?verbose=True")
    } else {
        format!("{api_uri}/{API_PATH}/job/{job_id}")
    }
}

/// DELETE /request/packages/<job_id>
pub(crate) fn delete_job(api_uri: &str, job_id: &JobId) -> String {
    format!("{api_uri}/{API_PATH}/job/{job_id}")
}

/// GET /job/packages/<type>/<name>/<version>
pub(crate) fn get_package_status(api_uri: &str, pkg: &PackageDescriptor) -> String {
    let name_escaped = pkg.name.replace("/", "~");
    let PackageDescriptor {
        package_type,
        version,
        ..
    } = pkg;
    format!("{api_uri}/{API_PATH}/job/packages/{package_type}/{name_escaped}/{version}")
}

pub(crate) fn get_project_details(api_uri: &str, pkg_id: &str) -> String {
    format!("{api_uri}/{API_PATH}/job/projects/name/{pkg_id}")
}
