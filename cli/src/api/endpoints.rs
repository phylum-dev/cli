/// API endpoint paths
use thiserror::Error as ThisError;
use url::{ParseError, Url};

use super::JobId;

const API_PATH: &str = "api/v0/";

// This wrapper provides important context to the user when their configuration
// has a bad URL. Without it, the message can be something like "empty host".
#[derive(Debug, ThisError)]
#[error("invalid API URL")]
pub struct BaseUriError(#[from] pub ParseError);

/// POST /data/jobs
pub fn post_submit_job(api_uri: &str) -> Result<Url, BaseUriError> {
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

/// POST /data/packages/submit
pub fn post_submit_package(api_uri: &str) -> Result<Url, BaseUriError> {
    Ok(get_api_path(api_uri)?.join("data/packages/submit")?)
}

/// GET /data/projects/<project_id>/history
pub fn get_project_history(api_uri: &str, project_id: &str) -> Result<Url, BaseUriError> {
    let mut url = get_api_path(api_uri)?;
    url.path_segments_mut()
        .unwrap()
        .pop_if_empty()
        .extend(["data", "projects", project_id, "history"]);
    Ok(url)
}

/// GET /groups/<group_name>/projects/<project_id>/history
pub fn get_group_project_history(
    api_uri: &str,
    project_id: &str,
    group_name: &str,
) -> Result<Url, BaseUriError> {
    let mut url = get_api_path(api_uri)?;
    url.path_segments_mut()
        .unwrap()
        .pop_if_empty()
        .extend(["groups", group_name, "projects", project_id, "history"]);
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

/// GET /groups
pub(crate) fn group_list(api_uri: &str) -> Result<Url, BaseUriError> {
    Ok(get_api_path(api_uri)?.join("groups")?)
}

/// POST /groups
pub(crate) fn group_create(api_uri: &str) -> Result<Url, BaseUriError> {
    Ok(get_api_path(api_uri)?.join("groups")?)
}

/// DELETE /groups/<groupName>
pub(crate) fn group_delete(api_uri: &str, group: &str) -> Result<Url, BaseUriError> {
    let mut url = get_api_path(api_uri)?;
    url.path_segments_mut().unwrap().pop_if_empty().extend(["groups", group]);
    Ok(url)
}

/// GET /groups/<groupName>/projects
pub fn group_project_summary(api_uri: &str, group: &str) -> Result<Url, BaseUriError> {
    let mut url = get_api_path(api_uri)?;
    url.path_segments_mut().unwrap().pop_if_empty().extend(["groups", group, "projects"]);
    Ok(url)
}

/// POST/DELETE /groups/<groupName>/members/<userEmail>
pub fn group_usermod(api_uri: &str, group: &str, user: &str) -> Result<Url, BaseUriError> {
    let mut url = get_api_path(api_uri)?;
    url.path_segments_mut().unwrap().pop_if_empty().extend(["groups", group, "members", user]);
    Ok(url)
}

/// GET /groups/<groupName>/members
pub fn group_members(api_uri: &str, group: &str) -> Result<Url, BaseUriError> {
    let mut url = get_api_path(api_uri)?;
    url.path_segments_mut().unwrap().pop_if_empty().extend(["groups", group, "members"]);
    Ok(url)
}

/// PUT /groups/<groupName>/owner/<userEmail>
pub fn set_owner(api_uri: &str, group: &str, owner: &str) -> Result<Url, BaseUriError> {
    let mut url = get_api_path(api_uri)?;
    url.path_segments_mut().unwrap().pop_if_empty().extend(["groups", group, "owner", owner]);
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
    fn put_submit_job_is_correct() {
        assert_eq!(
            post_submit_job(API_URI).unwrap().as_str(),
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
    fn post_submit_package_is_correct() {
        assert_eq!(
            post_submit_package(API_URI).unwrap().as_str(),
            format!("{API_URI}/{API_PATH}data/packages/submit"),
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
