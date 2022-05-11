/// API endpoint paths
use super::{JobId, PackageDescriptor};

const API_PATH: &str = "api/v0";

/// PUT /job
pub fn put_submit_package(api_uri: &str) -> String {
    format!("{api_uri}/{API_PATH}/job")
}

/// GET /health
pub fn get_ping(api_uri: &str) -> String {
    format!("{api_uri}/{API_PATH}/health")
}

/// GET /job/
pub fn get_all_jobs_status(api_uri: &str, limit: u32) -> String {
    format!("{api_uri}/{API_PATH}/job/?limit={limit}&verbose=1")
}

/// GET /job/<job_id>
pub fn get_job_status(api_uri: &str, job_id: &JobId, verbose: bool) -> String {
    if verbose {
        format!("{api_uri}/{API_PATH}/job/{job_id}?verbose=True")
    } else {
        format!("{api_uri}/{API_PATH}/job/{job_id}")
    }
}

/// GET /job/packages/<type>/<name>/<version>
pub fn get_package_status(api_uri: &str, pkg: &PackageDescriptor) -> String {
    let name_escaped = pkg.name.replace('/', "~");
    let PackageDescriptor {
        package_type,
        version,
        ..
    } = pkg;
    format!("{api_uri}/{API_PATH}/job/packages/{package_type}/{name_escaped}/{version}")
}

/// GET /job/projects/name/<pkg_id>
pub fn get_project_details(api_uri: &str, pkg_id: &str) -> String {
    format!("{api_uri}/{API_PATH}/job/projects/name/{pkg_id}")
}

/// GET /job/projects/overview
pub fn get_project_summary(api_uri: &str) -> String {
    format!("{api_uri}/{API_PATH}/job/projects/overview")
}

/// PUT /job/projects
pub fn put_create_project(api_uri: &str) -> String {
    format!("{api_uri}/{API_PATH}/job/projects")
}

/// GET /settings/current-user
pub(crate) fn get_user_settings(api_uri: &str) -> String {
    format!("{api_uri}/{API_PATH}/settings/current-user")
}

/// PUT /settings/current-user
pub(crate) fn put_user_settings(api_uri: &str) -> String {
    format!("{api_uri}/{API_PATH}/settings/current-user")
}
