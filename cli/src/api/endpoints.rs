/// API endpoint paths
use thiserror::Error as ThisError;
use url::{ParseError, Url};

use super::JobId;

/// Phylum API base path.
const API_PATH: &str = "api/v0/";

/// Locksmith API base path.
const LOCKSMITH_PATH: &str = "locksmith/v1/";

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

/// POST /data/jobs/{job_id}/policy/evaluate
pub fn get_job_status(api_uri: &str, job_id: &JobId) -> Result<Url, BaseUriError> {
    let mut url = get_api_path(api_uri)?;
    url.path_segments_mut().unwrap().pop_if_empty().extend([
        "data",
        "jobs",
        &job_id.to_string(),
        "policy",
        "evaluate",
    ]);
    Ok(url)
}

/// POST /data/jobs/{job_id}/policy/evaluate/raw
pub fn get_job_status_raw(api_uri: &str, job_id: &JobId) -> Result<Url, BaseUriError> {
    let mut url = get_api_path(api_uri)?;
    url.path_segments_mut().unwrap().pop_if_empty().extend([
        "data",
        "jobs",
        &job_id.to_string(),
        "policy",
        "evaluate",
        "raw",
    ]);
    Ok(url)
}

/// POST /data/packages/check
pub fn check_packages(api_uri: &str) -> Result<Url, BaseUriError> {
    Ok(get_api_path(api_uri)?.join("data/packages/check")?)
}

/// POST /data/packages/check/raw
pub fn check_packages_raw(api_uri: &str) -> Result<Url, BaseUriError> {
    Ok(get_api_path(api_uri)?.join("data/packages/check/raw")?)
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

/// GET /projects
pub fn projects(api_uri: &str) -> Result<Url, BaseUriError> {
    Ok(get_api_path(api_uri)?.join("projects")?)
}

/// POST /data/projects
pub fn create_project(api_uri: &str) -> Result<Url, BaseUriError> {
    Ok(get_api_path(api_uri)?.join("data/projects")?)
}

/// GET/PUT/DELETE /data/projects/<project_id>
pub fn project(api_uri: &str, project_id: &str) -> Result<Url, BaseUriError> {
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

/// GET /organizations
pub fn orgs(api_uri: &str) -> Result<Url, BaseUriError> {
    Ok(get_api_path(api_uri)?.join("organizations")?)
}

/// GET /organizations/<orgName>/members
pub fn org_members(api_uri: &str, org: &str) -> Result<Url, BaseUriError> {
    let mut url = get_api_path(api_uri)?;
    url.path_segments_mut().unwrap().pop_if_empty().extend(["organizations", org, "members"]);
    Ok(url)
}

/// DELETE /organizations/<orgName>/members/<email>
pub fn org_member_remove(api_uri: &str, org: &str, email: &str) -> Result<Url, BaseUriError> {
    let mut url = get_api_path(api_uri)?;
    url.path_segments_mut().unwrap().pop_if_empty().extend([
        "organizations",
        org,
        "members",
        email,
    ]);
    Ok(url)
}

/// GET/POST /organizations/<orgName>/groups
pub fn org_groups(api_uri: &str, org_name: &str) -> Result<Url, BaseUriError> {
    let mut url = get_api_path(api_uri)?;
    url.path_segments_mut().unwrap().pop_if_empty().extend(["organizations", org_name, "groups"]);
    Ok(url)
}

/// DELETE /organizations/<orgName>/groups/<groupName>
pub fn org_groups_delete(
    api_uri: &str,
    org_name: &str,
    group_name: &str,
) -> Result<Url, BaseUriError> {
    let mut url = get_api_path(api_uri)?;
    url.path_segments_mut().unwrap().pop_if_empty().extend([
        "organizations",
        org_name,
        "groups",
        group_name,
    ]);
    Ok(url)
}

/// Aviary activity endpoint.
pub fn firewall_log(api_uri: &str) -> Result<Url, BaseUriError> {
    Ok(get_firewall_path(api_uri)?.join("activity")?)
}

/// GET /organizations/<orgName>/groups/<groupName>/preferences.
pub fn org_group_preferences(
    api_uri: &str,
    org_name: &str,
    group_name: &str,
) -> Result<Url, BaseUriError> {
    let mut url = get_api_path(api_uri)?;
    url.path_segments_mut().unwrap().pop_if_empty().extend([
        "organizations",
        org_name,
        "groups",
        group_name,
        "preferences",
    ]);
    Ok(url)
}

/// GET /preferences/group/<groupName>
pub fn group_preferences(api_uri: &str, group_name: &str) -> Result<Url, BaseUriError> {
    let mut url = get_api_path(api_uri)?;
    url.path_segments_mut().unwrap().pop_if_empty().extend(["preferences", "group", group_name]);
    Ok(url)
}

/// GET /preferences/project/<projectId>
pub fn project_preferences(api_uri: &str, project_id: &str) -> Result<Url, BaseUriError> {
    let mut url = get_api_path(api_uri)?;
    url.path_segments_mut().unwrap().pop_if_empty().extend(["preferences", "project", project_id]);
    Ok(url)
}

/// POST /organizations/<orgName>/groups/<groupName>/suppress.
pub fn org_group_suppress(
    api_uri: &str,
    org_name: &str,
    group_name: &str,
) -> Result<Url, BaseUriError> {
    let mut url = get_api_path(api_uri)?;
    url.path_segments_mut().unwrap().pop_if_empty().extend([
        "organizations",
        org_name,
        "groups",
        group_name,
        "suppress",
    ]);
    Ok(url)
}

/// POST /preferences/group/<groupName>/suppress.
pub fn group_suppress(api_uri: &str, group_name: &str) -> Result<Url, BaseUriError> {
    let mut url = get_api_path(api_uri)?;
    url.path_segments_mut().unwrap().pop_if_empty().extend([
        "preferences",
        "group",
        group_name,
        "suppress",
    ]);
    Ok(url)
}

/// POST /preferences/project/<projectId>/suppress.
pub fn project_suppress(api_uri: &str, project_id: &str) -> Result<Url, BaseUriError> {
    let mut url = get_api_path(api_uri)?;
    url.path_segments_mut().unwrap().pop_if_empty().extend([
        "preferences",
        "project",
        project_id,
        "suppress",
    ]);
    Ok(url)
}

/// POST /organizations/<orgName>/groups/<groupName>/unsuppress.
pub fn org_group_unsuppress(
    api_uri: &str,
    org_name: &str,
    group_name: &str,
) -> Result<Url, BaseUriError> {
    let mut url = get_api_path(api_uri)?;
    url.path_segments_mut().unwrap().pop_if_empty().extend([
        "organizations",
        org_name,
        "groups",
        group_name,
        "unsuppress",
    ]);
    Ok(url)
}

/// POST /preferences/group/<groupName>/unsuppress.
pub fn group_unsuppress(api_uri: &str, group_name: &str) -> Result<Url, BaseUriError> {
    let mut url = get_api_path(api_uri)?;
    url.path_segments_mut().unwrap().pop_if_empty().extend([
        "preferences",
        "group",
        group_name,
        "unsuppress",
    ]);
    Ok(url)
}

/// POST /preferences/project/<projectId>/unsuppress.
pub fn project_unsuppress(api_uri: &str, project_id: &str) -> Result<Url, BaseUriError> {
    let mut url = get_api_path(api_uri)?;
    url.path_segments_mut().unwrap().pop_if_empty().extend([
        "preferences",
        "project",
        project_id,
        "unsuppress",
    ]);
    Ok(url)
}

/// GET /.well-known/openid-configuration
pub fn oidc_discovery(api_uri: &str) -> Result<Url, BaseUriError> {
    Ok(get_api_path(api_uri)?.join(".well-known/openid-configuration")?)
}

/// GET /.well-known/locksmith-configuration
pub fn locksmith_discovery(api_uri: &str) -> Result<Url, BaseUriError> {
    Ok(get_locksmith_path(api_uri)?.join(".well-known/locksmith-configuration")?)
}

/// GET /tokens
pub fn list_tokens(api_uri: &str) -> Result<Url, BaseUriError> {
    Ok(get_locksmith_path(api_uri)?.join("tokens")?)
}

/// POST /revoke
pub fn revoke_token(api_uri: &str) -> Result<Url, BaseUriError> {
    Ok(get_locksmith_path(api_uri)?.join("revoke")?)
}

/// POST /reachability/vulnerabilities
pub fn vulnreach(api_uri: &str) -> Result<Url, BaseUriError> {
    Ok(parse_base_url(api_uri)?.join("reachability/vulnerabilities")?)
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

fn get_locksmith_path(api_uri: &str) -> Result<Url, BaseUriError> {
    Ok(parse_base_url(api_uri)?.join(LOCKSMITH_PATH)?)
}

fn get_firewall_path(api_uri: &str) -> Result<Url, BaseUriError> {
    let mut api_path = parse_base_url(api_uri)?;
    let host = api_path.host_str().ok_or(ParseError::EmptyHost)?;
    let host = host.replacen("api.", "aviary.", 1);
    api_path.set_host(Some(&host))?;
    Ok(api_path)
}

#[cfg(test)]
mod test {
    use super::*;

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
}
