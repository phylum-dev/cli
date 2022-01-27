use super::common::API_PATH;
use super::{JobId, PackageDescriptor};

/// PUT /job
pub(crate) fn put_submit_package(api_uri: &str) -> String {
    format!("{api_uri}/{API_PATH}/job")
}
// impl RestPath<()> for SubmitPackageRequest {
//     fn get_path(_: ()) -> Result<String, Error> {
//         Ok(format!("{}/job", API_PATH))
//     }
// }

/// GET /job/auth_status
pub(crate) fn get_auth_status(api_uri: &str) -> String {
    format!("{api_uri}/{API_PATH}/job/auth_status")
}
// impl RestPath<()> for AuthStatusResponse {
//     fn get_path(_: ()) -> Result<String, Error> {
//         Ok(format!("{}/job/auth_status", API_PATH))
//     }
// }

/// GET /job/heartbeat
// TODO Deprecate
pub(crate) fn get_ping(api_uri: &str) -> String {
    format!("{api_uri}/{API_PATH}/job/heartbeat")
}
// impl RestPath<()> for PingResponse {
//     fn get_path(_: ()) -> Result<String, Error> {
//         Ok(format!("{}/job/heartbeat", API_PATH))
//     }
// }

/// GET /job/
pub(crate) fn get_all_jobs_status(api_uri: &str, limit: u32) -> String {
    format!("{api_uri}/{API_PATH}/job/?limit={limit}&verbose=1")
}
// impl RestPath<u32> for AllJobsStatusResponse {
//     fn get_path(limit: u32) -> Result<String, Error> {
//         Ok(format!("{}/job/?limit={}&verbose=1", API_PATH, limit))
//     }
// }

/// Get /job/<job_id> summary
pub(crate) fn get_job_status(api_uri: &str, job_id: &JobId, verbose: bool) -> String {
    if verbose {
        format!("{api_uri}/{API_PATH}/job/{job_id}?verbose=True")
    } else {
        format!("{api_uri}/{API_PATH}/job/{job_id}")
    }
}
// impl<'a> RestPath<JobId> for JobStatusResponse<PackageStatus> {
//     fn get_path(job_id: JobId) -> Result<String, Error> {
//         Ok(format!("{}/job/{}", API_PATH, job_id))
//     }
// }

/// Get /job/<job_id> verbose
// impl<'a> RestPath<JobId> for JobStatusResponse<PackageStatusExtended> {
//     fn get_path(job_id: JobId) -> Result<String, Error> {
//         Ok(format!("{}/job/{}?verbose=True", API_PATH, job_id))
//     }
// }

/// DELETE /request/packages/<job_id>
pub(crate) fn delete_job(api_uri: &str, job_id: &JobId) -> String {
    format!("{api_uri}/{API_PATH}/job/{job_id}")
}
// impl<'a> RestPath<JobId> for CancelJobResponse {
//     fn get_path(job_id: JobId) -> Result<String, Error> {
//         Ok(format!("{}/job/{}", API_PATH, job_id))
//     }
// }

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
// impl<'a> RestPath<PackageDescriptor> for PackageStatusExtended {
//     fn get_path(pkg: PackageDescriptor) -> Result<String, Error> {
//         let name_escaped = pkg.name.replace("/", "~");
//         let endpoint = format!("{}/{}/{}", pkg.package_type, name_escaped, pkg.version);
//         Ok(format!("{}/job/packages/{}", API_PATH, endpoint))
//     }
// }

pub(crate) fn get_project_details(api_uri: &str, pkg_id: &str) -> String {
    format!("{api_uri}/{API_PATH}/job/projects/name/{pkg_id}")
}
// impl RestPath<&str> for ProjectDetailsResponse {
//     fn get_path(pkg_id: &str) -> Result<String, Error> {
//         Ok(format!("{}/job/projects/name/{}", API_PATH, pkg_id))
//     }
// }
