/// API endpoint paths
use thiserror::Error as ThisError;
use url::{ParseError, Url};

use super::{JobId, PackageDescriptor};

const API_PATH: &str = "api/v0/";

// This wrapper provides important context to the user when their configuration
// has a bad URL. Without it, the message can be something like "Error creating
// client" caused by "empty host".
#[derive(Debug, ThisError)]
#[error("invalid API URL")]
pub struct BaseUriError(#[from] pub ParseError);

/// PUT /data/jobs
pub fn put_submit_package(api_uri: &str) -> Result<Url, BaseUriError> {
    Ok(get_api_path(api_uri)?.join("data/jobs")?)
}

/// GET /health
pub fn get_ping(api_uri: &str) -> Result<Url, BaseUriError> {
    Ok(get_api_path(api_uri)?.join("health")?)
}

/// GET /data/jobs/
pub fn get_all_jobs_status(api_uri: &str, limit: u32) -> Result<Url, BaseUriError> {
    Ok(get_api_path(api_uri)?.join(&format!("data/jobs/?limit={limit}&verbose=1"))?)
}

/// GET /data/jobs/<job_id>
pub fn get_job_status(api_uri: &str, job_id: &JobId, verbose: bool) -> Result<Url, BaseUriError> {
    let mut url = get_api_path(api_uri)?;

    // Unwrap is okay because get_api_path only returns URLs that can be base URLs.
    url.path_segments_mut().unwrap().pop_if_empty().extend(["data", "jobs", &job_id.to_string()]);

    if verbose {
        url.query_pairs_mut().append_pair("verbose", "True");
    }

    Ok(url)
}

/// GET /data/packages/<type>/<name>/<version>
pub fn get_package_status(api_uri: &str, pkg: &PackageDescriptor) -> Result<Url, BaseUriError> {
    let mut url = get_api_path(api_uri)?;

    let PackageDescriptor { name, package_type, version, .. } = pkg;
    url.path_segments_mut().unwrap().pop_if_empty().extend([
        "data",
        "packages",
        &package_type.to_string(),
        name,
        version,
    ]);

    Ok(url)
}

/// GET /data/projects/name/<pkg_id>
pub fn get_project_details(api_uri: &str, pkg_id: &str) -> Result<Url, BaseUriError> {
    let mut url = get_api_path(api_uri)?;
    url.path_segments_mut().unwrap().pop_if_empty().extend(["data", "projects", "name", pkg_id]);
    Ok(url)
}

/// GET /data/projects/overview
pub fn get_project_summary(api_uri: &str) -> Result<Url, BaseUriError> {
    Ok(get_api_path(api_uri)?.join("data/projects/overview")?)
}

/// POST /data/projects
pub fn post_create_project(api_uri: &str) -> Result<Url, BaseUriError> {
    Ok(get_api_path(api_uri)?.join("data/projects")?)
}

/// DELETE /data/projects/<project_id>
pub fn delete_project(api_uri: &str, project_id: &str) -> Result<Url, BaseUriError> {
    let mut url = get_api_path(api_uri)?;
    url.path_segments_mut().unwrap().pop_if_empty().extend(["data", "projects", project_id]);
    Ok(url)
}

/// GET /settings/current-user
pub(crate) fn get_user_settings(api_uri: &str) -> Result<Url, BaseUriError> {
    Ok(get_api_path(api_uri)?.join("settings/current-user")?)
}

/// PUT /settings/current-user
pub(crate) fn put_user_settings(api_uri: &str) -> Result<Url, BaseUriError> {
    Ok(get_api_path(api_uri)?.join("settings/current-user")?)
}

/// GET /groups
pub(crate) fn group_list(api_uri: &str) -> Result<Url, BaseUriError> {
    Ok(get_api_path(api_uri)?.join("groups")?)
}

/// POST /groups
pub(crate) fn group_create(api_uri: &str) -> Result<Url, BaseUriError> {
    Ok(get_api_path(api_uri)?.join("groups")?)
}

/// GET /groups/<groupName>/projects
pub fn group_project_summary(api_uri: &str, group: &str) -> Result<Url, BaseUriError> {
    let mut url = get_api_path(api_uri)?;
    url.path_segments_mut().unwrap().pop_if_empty().extend(["groups", group, "projects"]);
    Ok(url)
}

/// GET /.well-known/openid-configuration
pub fn oidc_discovery(api_uri: &str) -> Result<Url, BaseUriError> {
    Ok(get_api_path(api_uri)?.join(".well-known/openid-configuration")?)
}

fn parse_base_url(api_uri: &str) -> Result<Url, BaseUriError> {
    let mut url = Url::parse(api_uri)?;

    // Ensure the path can be a base and ends with a slash so it can be safely
    // joined to. If we don't do this, https://example.com/a and https://example.com/a/ are different.
    url.path_segments_mut()
        .map_err(|_| ParseError::RelativeUrlWithCannotBeABaseBase)?
        .pop_if_empty()
        .push("");

    // Ensure there are no extra bits.
    url.set_query(None);
    url.set_fragment(None);

    Ok(url)
}

fn get_api_path(api_uri: &str) -> Result<Url, BaseUriError> {
    Ok(parse_base_url(api_uri)?.join(API_PATH)?)
}

#[cfg(test)]
mod test {
    use phylum_types::types::package::PackageType;

    use super::*;

    const API_URI: &str = "https://example.com/a";

    #[test]
    fn get_api_path_returns_api_base() {
        assert_eq!(
            get_api_path("https://example.com/").unwrap().as_str(),
            "https://example.com/api/v0/",
        );
        assert_eq!(
            get_api_path("https://example.com/a").unwrap().as_str(),
            "https://example.com/a/api/v0/",
        );

        // Maybe an error should be reported in this case instead of stripping the
        // extras.
        assert_eq!(
            get_api_path("https://example.com/search?q=invalid#search").unwrap().as_str(),
            "https://example.com/search/api/v0/",
        );
    }

    #[test]
    fn put_submit_package_is_correct() {
        assert_eq!(
            put_submit_package(API_URI).unwrap().as_str(),
            format!("{API_URI}/{API_PATH}data/jobs"),
        );
    }

    #[test]
    fn get_ping_is_correct() {
        assert_eq!(get_ping(API_URI).unwrap().as_str(), format!("{API_URI}/{API_PATH}health"),);
    }

    #[test]
    fn get_all_jobs_status_is_correct() {
        assert_eq!(
            get_all_jobs_status(API_URI, 123).unwrap().as_str(),
            format!("{API_URI}/{API_PATH}data/jobs/?limit=123&verbose=1"),
        );
    }

    #[test]
    fn get_job_status_is_correct() {
        const JOB_ID: &str = "e00ad8fd-73b2-4259-b4ed-a188a405f5eb";
        let job_id = JobId::parse_str(JOB_ID).unwrap();

        assert_eq!(
            get_job_status(API_URI, &job_id, false).unwrap().as_str(),
            format!("{API_URI}/{API_PATH}data/jobs/{JOB_ID}"),
        );
        assert_eq!(
            get_job_status(API_URI, &job_id, true).unwrap().as_str(),
            format!("{API_URI}/{API_PATH}data/jobs/{JOB_ID}?verbose=True"),
        );
    }

    #[test]
    fn get_package_status_is_correct() {
        let pkg = PackageDescriptor {
            name: "@acme/widgets".to_owned(),
            version: "1.2.3-final+build4".to_owned(),
            package_type: PackageType::Npm,
        };
        assert_eq!(
            get_package_status(API_URI, &pkg).unwrap().as_str(),
            format!("{API_URI}/{API_PATH}data/packages/npm/@acme%2Fwidgets/1.2.3-final+build4"),
        );
    }

    #[test]
    fn get_project_details_is_correct() {
        assert_eq!(
            get_project_details(API_URI, "acme/widgets").unwrap().as_str(),
            format!("{API_URI}/{API_PATH}data/projects/name/acme%2Fwidgets"),
        );
    }

    #[test]
    fn get_project_summary_is_correct() {
        assert_eq!(
            get_project_summary(API_URI).unwrap().as_str(),
            format!("{API_URI}/{API_PATH}data/projects/overview"),
        );
    }

    #[test]
    fn put_create_project_is_correct() {
        assert_eq!(
            post_create_project(API_URI).unwrap().as_str(),
            format!("{API_URI}/{API_PATH}data/projects"),
        );
    }

    #[test]
    fn delete_project_is_correct() {
        assert_eq!(
            delete_project(API_URI, "12345678-90ab-cdef-1234-567890abcdef").unwrap().as_str(),
            format!("{API_URI}/{API_PATH}data/projects/12345678-90ab-cdef-1234-567890abcdef"),
        );
    }

    #[test]
    fn get_user_settings_is_correct() {
        assert_eq!(
            get_user_settings(API_URI).unwrap().as_str(),
            format!("{API_URI}/{API_PATH}settings/current-user"),
        );
    }

    #[test]
    fn put_user_settings_is_correct() {
        assert_eq!(
            get_user_settings(API_URI).unwrap().as_str(),
            format!("{API_URI}/{API_PATH}settings/current-user"),
        );
    }

    #[test]
    fn group_list_is_correct() {
        assert_eq!(group_list(API_URI).unwrap().as_str(), format!("{API_URI}/{API_PATH}groups"),);
    }

    #[test]
    fn group_create_is_correct() {
        assert_eq!(group_create(API_URI).unwrap().as_str(), format!("{API_URI}/{API_PATH}groups"),);
    }

    #[test]
    fn group_project_summary_is_correct() {
        assert_eq!(
            group_project_summary(API_URI, "acme/misc. projects").unwrap().as_str(),
            format!("{API_URI}/{API_PATH}groups/acme%2Fmisc.%20projects/projects"),
        );
    }

    #[test]
    fn oidc_discovery_is_correct() {
        assert_eq!(
            oidc_discovery(API_URI).unwrap().as_str(),
            format!("{API_URI}/{API_PATH}.well-known/openid-configuration"),
        );
    }
}
