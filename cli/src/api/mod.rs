use std::borrow::Cow;
use std::collections::HashSet;
use std::time::Duration;

use anyhow::{anyhow, Context};
use phylum_types::types::common::{JobId, ProjectId};
use phylum_types::types::group::{
    CreateGroupRequest, CreateGroupResponse, ListGroupMembersResponse,
};
use phylum_types::types::job::{AllJobsStatusResponse, SubmitPackageResponse};
use phylum_types::types::package::PackageDescriptor;
use phylum_types::types::project::CreateProjectResponse;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{Client, IntoUrl, Method, StatusCode};
use serde::de::{DeserializeOwned, IgnoredAny};
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;
#[cfg(feature = "vulnreach")]
use vulnreach_types::{Job, Vulnerability};

use crate::api::endpoints::BaseUriError;
use crate::app::USER_AGENT;
use crate::auth::jwt::RealmRole;
use crate::auth::{
    fetch_locksmith_server_settings, handle_auth_flow, jwt, renew_access_token, AuthAction,
    UserInfo,
};
use crate::config::{AuthInfo, Config};
use crate::types::{
    AddOrgUserRequest, AnalysisPackageDescriptor, ApiOrgGroup, CreateProjectRequest,
    GetProjectResponse, HistoryJob, ListUserGroupsResponse, OrgGroupsResponse, OrgMembersResponse,
    OrgsResponse, PackageSpecifier, PackageSubmitResponse, Paginated, PingResponse,
    PolicyEvaluationRequest, PolicyEvaluationResponse, PolicyEvaluationResponseRaw,
    ProjectListEntry, RevokeTokenRequest, SubmitPackageRequest, UpdateProjectRequest, UserToken,
};

pub mod endpoints;

type Result<T> = std::result::Result<T, PhylumApiError>;

pub struct PhylumApi {
    roles: HashSet<RealmRole>,
    config: Config,
    client: Client,
}

/// Phylum Api Error type
#[derive(ThisError, Debug)]
pub enum PhylumApiError {
    #[error("Error invoking REST endpoint")]
    ReqwestError {
        #[from]
        source: reqwest::Error,
    },
    #[error(transparent)]
    BaseUri(#[from] BaseUriError),
    #[error(transparent)]
    Response(#[from] ResponseError),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl PhylumApiError {
    pub fn status(&self) -> Option<StatusCode> {
        match self {
            PhylumApiError::ReqwestError { source } => source.status(),
            PhylumApiError::Response(ResponseError { code, .. }) => Some(*code),
            _ => None,
        }
    }
}

/// Non-successful request response.
#[derive(ThisError, Debug)]
#[error("HTTP request error ({code}):\n\n{details}")]
pub struct ResponseError {
    pub code: StatusCode,
    pub details: String,
}

/// The guts of an API JSON error
#[derive(Deserialize)]
struct ApiJsonErrorInner {
    error_id: String,
    description: String,
}

/// A JSON error returned by the Phylum API
#[derive(Deserialize)]
struct ApiJsonError {
    error: ApiJsonErrorInner,
}

impl PhylumApi {
    /// Create a phylum API client using the given Auth configuration, api url
    /// and request timeout. If in the process of creating the client,
    /// credentials must be obtained, the auth_info struct will be updated
    /// with the new information. It is the duty of the calling code to save
    /// any changes
    pub async fn new(mut config: Config, request_timeout: Option<u64>) -> Result<Self> {
        // Do we have a refresh token?
        let ignore_certs = config.ignore_certs();
        let refresh_token = match config.auth_info.offline_access() {
            Some(refresh_token) => refresh_token.clone(),
            None => {
                let refresh_token = handle_auth_flow(
                    AuthAction::Login,
                    None,
                    None,
                    ignore_certs,
                    &config.connection.uri,
                )
                .await
                .context("User login failed")?;
                config.auth_info.set_offline_access(refresh_token.clone());
                refresh_token
            },
        };

        let access_token = renew_access_token(&refresh_token, ignore_certs, &config.connection.uri)
            .await
            .context("Token refresh failed")?;

        let mut headers = HeaderMap::new();
        // the cli runs a command or a few short commands then exits, so we do
        // not need to worry about refreshing the access token. We just set it
        // here and be done.
        headers.insert(
            "Authorization",
            HeaderValue::from_str(&format!("Bearer {}", access_token)).unwrap(),
        );
        headers.insert("Accept", HeaderValue::from_str("application/json").unwrap());

        let client = Client::builder()
            .user_agent(USER_AGENT.as_str())
            .timeout(Duration::from_secs(request_timeout.unwrap_or(u64::MAX)))
            .danger_accept_invalid_certs(ignore_certs)
            .default_headers(headers)
            .build()?;

        // Try to parse token's roles.
        let roles = jwt::user_roles(access_token.as_str()).unwrap_or_default();

        Ok(Self { config, client, roles })
    }

    async fn get<T: DeserializeOwned, U: IntoUrl>(&self, path: U) -> Result<T> {
        self.send_request(Method::GET, path, None::<()>).await
    }

    async fn delete<T: DeserializeOwned, U: IntoUrl>(&self, path: U) -> Result<T> {
        self.send_request(Method::DELETE, path, None::<()>).await
    }

    async fn post<T: serde::de::DeserializeOwned, S: serde::Serialize, U: IntoUrl>(
        &self,
        path: U,
        s: S,
    ) -> Result<T> {
        self.send_request(Method::POST, path, Some(s)).await
    }

    async fn put<T: serde::de::DeserializeOwned, S: serde::Serialize, U: IntoUrl>(
        &self,
        path: U,
        s: S,
    ) -> Result<T> {
        self.send_request(Method::PUT, path, Some(s)).await
    }

    async fn send_request<T: DeserializeOwned, B: Serialize, U: IntoUrl>(
        &self,
        method: Method,
        path: U,
        body: Option<B>,
    ) -> Result<T> {
        let body = self.send_request_raw(method, path, body).await?;
        serde_json::from_str::<T>(&body).map_err(|e| PhylumApiError::Other(e.into()))
    }

    async fn send_request_raw<B: Serialize, U: IntoUrl>(
        &self,
        method: Method,
        path: U,
        body: Option<B>,
    ) -> Result<String> {
        let mut request = self.client.request(method, path);
        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request.send().await?;
        let status_code = response.status();
        let body = response.text().await?;

        if !status_code.is_success() {
            let details = if let Ok(err) = serde_json::from_str::<ApiJsonError>(&body) {
                log::debug!("Error ID: {}", err.error.error_id);
                err.error.description
            } else {
                body
            };
            let err = ResponseError { details, code: status_code };

            return Err(err.into());
        }

        Ok(body)
    }

    /// update auth info by forcing the login flow, using the given Auth
    /// configuration. The auth_info struct will be updated with the new
    /// credentials. It is the duty of the calling code to save any changes
    pub async fn login(
        mut auth_info: AuthInfo,
        token_name: Option<String>,
        ignore_certs: bool,
        api_uri: &str,
        reauth: bool,
    ) -> Result<AuthInfo> {
        let action = if reauth { AuthAction::Reauth } else { AuthAction::Login };
        let refresh_token =
            handle_auth_flow(action, token_name, None, ignore_certs, api_uri).await?;
        auth_info.set_offline_access(refresh_token);
        Ok(auth_info)
    }

    /// update auth info by forcing the registration flow, using the given Auth
    /// configuration. The auth_info struct will be updated with the new
    /// credentials. It is the duty of the calling code to save any changes
    pub async fn register(
        mut auth_info: AuthInfo,
        token_name: Option<String>,
        ignore_certs: bool,
        api_uri: &str,
    ) -> Result<AuthInfo> {
        let refresh_token =
            handle_auth_flow(AuthAction::Register, token_name, None, ignore_certs, api_uri).await?;
        auth_info.set_offline_access(refresh_token);
        Ok(auth_info)
    }

    /// Ping the system and verify it's up
    pub async fn ping(&self) -> Result<String> {
        Ok(self
            .get::<PingResponse, _>(endpoints::get_ping(&self.config.connection.uri)?)
            .await?
            .response)
    }

    /// Get information about the authenticated user
    pub async fn user_info(&self) -> Result<UserInfo> {
        let locksmith_settings = fetch_locksmith_server_settings(
            self.config.ignore_certs(),
            &self.config.connection.uri,
        )
        .await?;
        self.get(locksmith_settings.userinfo_endpoint).await
    }

    /// Create a new project
    pub async fn create_project(
        &self,
        name: impl Into<String>,
        org: Option<&str>,
        group: Option<String>,
        repository_url: Option<String>,
    ) -> Result<ProjectId> {
        let group_name = match (org, group) {
            (Some(org), Some(group)) => Some(format!("{org}/{group}")),
            (None, Some(group)) => Some(group),
            (Some(_), None) | (None, None) => None,
        };

        let url = endpoints::create_project(&self.config.connection.uri)?;
        let body = CreateProjectRequest {
            repository_url,
            group_name,
            default_label: None,
            name: name.into(),
        };
        let response: CreateProjectResponse = self.post(url, body).await?;
        Ok(response.id)
    }

    /// Update an existing new project.
    pub async fn update_project(
        &self,
        project_id: &str,
        org: Option<String>,
        group: Option<String>,
        name: impl Into<String>,
        repository_url: Option<String>,
        default_label: Option<String>,
    ) -> Result<ProjectId> {
        let group_name = match (org, group) {
            (Some(org), Some(group)) => Some(format!("{org}/{group}")),
            (None, Some(group)) => Some(group),
            (Some(_), None) | (None, None) => None,
        };

        let url = endpoints::project(&self.config.connection.uri, project_id)?;
        let body =
            UpdateProjectRequest { repository_url, default_label, name: name.into(), group_name };
        let response: CreateProjectResponse = self.put(url, body).await?;
        Ok(response.id)
    }

    /// Delete a project
    pub async fn delete_project(&self, project_id: ProjectId) -> Result<()> {
        let _: IgnoredAny = self
            .delete(endpoints::project(&self.config.connection.uri, &project_id.to_string())?)
            .await?;
        Ok(())
    }

    /// Get all projects.
    ///
    /// If a group is passed, only projects of that group will be returned.
    /// Otherwise all projects, including group projects, will be returned.
    ///
    /// The project name filter does not require an exact match, it is
    /// equivalent to filtering with [`str::contains`].
    pub async fn get_projects(
        &self,
        org: Option<&str>,
        group: Option<&str>,
        name_filter: Option<&str>,
    ) -> Result<Vec<ProjectListEntry>> {
        let mut uri = endpoints::projects(&self.config.connection.uri)?;

        // Add filter query parameters.
        match (org, group) {
            (Some(org), Some(group)) => {
                uri.query_pairs_mut().append_pair("filter.group", &format!("{org}/{group}"));
            },
            (Some(org), None) => {
                uri.query_pairs_mut().append_pair("filter.organization", org);
            },
            (None, Some(group)) => {
                uri.query_pairs_mut().append_pair("filter.group", group);
            },
            (None, None) => (),
        }
        if let Some(name_filter) = name_filter {
            uri.query_pairs_mut().append_pair("filter.name", name_filter);
        }

        // Set maximum pagination size, since we want everything anyway.
        uri.query_pairs_mut().append_pair("paginate.limit", "100");

        let mut projects: Vec<ProjectListEntry> = Vec::new();
        loop {
            // Update the pagination cursor point.
            let mut uri = uri.clone();
            if let Some(project) = projects.last() {
                uri.query_pairs_mut().append_pair("paginate.cursor", &project.id.to_string());
            }

            // Get next page of projects.
            let mut page: Paginated<ProjectListEntry> = self.get(uri).await?;
            projects.append(&mut page.values);

            // Keep paginating until there's nothing left.
            if !page.has_more {
                break;
            }
        }

        Ok(projects)
    }

    /// Submit a new request to the system
    pub async fn submit_request(
        &self,
        package_list: &[AnalysisPackageDescriptor],
        project: ProjectId,
        label: Option<String>,
        group_name: Option<String>,
    ) -> Result<JobId> {
        let req = SubmitPackageRequest {
            packages: package_list.to_vec(),
            is_user: true,
            project,
            label: label.unwrap_or_else(|| "uncategorized".to_string()),
            group_name,
        };
        log::debug!("==> Sending package submission: {:?}", req);
        let resp: SubmitPackageResponse =
            self.post(endpoints::post_submit_job(&self.config.connection.uri)?, req).await?;
        Ok(resp.job_id)
    }

    /// Get the status of a previously submitted job.
    pub async fn get_job_status(
        &self,
        job_id: &JobId,
        ignored: impl Into<Vec<PackageDescriptor>>,
    ) -> Result<PolicyEvaluationResponse> {
        let body = PolicyEvaluationRequest { ignored_packages: ignored.into() };
        self.post(endpoints::get_job_status(&self.config.connection.uri, job_id)?, body).await
    }

    /// Get the status of a previously submitted job.
    pub async fn get_job_status_raw(
        &self,
        job_id: &JobId,
        ignored: impl Into<Vec<PackageDescriptor>>,
    ) -> Result<PolicyEvaluationResponseRaw> {
        let body = PolicyEvaluationRequest { ignored_packages: ignored.into() };
        self.post(endpoints::get_job_status_raw(&self.config.connection.uri, job_id)?, body).await
    }

    /// Check a set of packages against the default policy
    pub async fn check_packages(
        &self,
        package_list: &[PackageDescriptor],
    ) -> Result<PolicyEvaluationResponse> {
        self.post(endpoints::check_packages(&self.config.connection.uri)?, package_list).await
    }

    /// Check a set of packages against the default policy.
    pub async fn check_packages_raw(
        &self,
        package_list: &[PackageDescriptor],
    ) -> Result<PolicyEvaluationResponseRaw> {
        self.post(endpoints::check_packages_raw(&self.config.connection.uri)?, package_list).await
    }

    /// Get the status of all jobs
    pub async fn get_status(&self) -> Result<AllJobsStatusResponse> {
        self.get(endpoints::get_all_jobs_status(&self.config.connection.uri, 30)?).await
    }

    /// Get project's job history.
    pub async fn get_project_history(
        &self,
        project_name: &str,
        group: Option<Group>,
    ) -> Result<Vec<HistoryJob>> {
        let project_id = self.get_project_id(project_name, group).await?.to_string();
        let url = endpoints::get_project_history(&self.config.connection.uri, &project_id)?;
        self.get(url).await
    }

    /// Resolve a Project Name to a Project ID
    pub async fn get_project_id(
        &self,
        project_name: &str,
        group: Option<Group>,
    ) -> Result<ProjectId> {
        let (org, group, combined_format) = group.as_ref().map_or((None, None, None), |group| {
            (group.org(), Some(group.name()), Some(group.combined_format().into_owned()))
        });

        let projects = self.get_projects(org, group, Some(project_name)).await?;

        projects
            .iter()
            .find(|project| project.name == project_name && project.group_name == combined_format)
            .ok_or_else(|| anyhow!("No project found with name {:?}", project_name).into())
            .map(|project| project.id)
    }

    /// Get a project using its ID and group name.
    pub async fn get_project(&self, project_id: &str) -> Result<GetProjectResponse> {
        let url = endpoints::project(&self.config.connection.uri, project_id)?;
        self.get(url).await
    }

    /// Submit a single package
    pub async fn submit_package(&self, pkg: &PackageSpecifier) -> Result<PackageSubmitResponse> {
        self.post(endpoints::post_submit_package(&self.config.connection.uri)?, pkg).await
    }

    /// Get all groups the user is part of.
    pub async fn get_groups_list(&self) -> Result<ListUserGroupsResponse> {
        self.get(endpoints::group_list(&self.config.connection.uri)?).await
    }

    /// Create a new group.
    pub async fn create_group(&self, group_name: &str) -> Result<CreateGroupResponse> {
        let group = CreateGroupRequest { group_name: group_name.into() };
        self.post(endpoints::group_create(&self.config.connection.uri)?, group).await
    }

    /// Delete an existing group.
    pub async fn delete_group(&self, group_name: &str) -> Result<()> {
        let url = endpoints::group_delete(&self.config.connection.uri, group_name)?;
        self.send_request_raw(Method::DELETE, url, None::<()>).await?;
        Ok(())
    }

    /// Get members of a group.
    pub async fn group_members(&self, group_name: &str) -> Result<ListGroupMembersResponse> {
        let url = endpoints::group_members(&self.config.connection.uri, group_name)?;
        self.get(url).await
    }

    /// Add user to a group.
    pub async fn group_add(&self, group_name: &str, user_email: &str) -> Result<()> {
        let url = endpoints::group_usermod(&self.config.connection.uri, group_name, user_email)?;
        self.send_request_raw(Method::POST, url, None::<()>).await?;
        Ok(())
    }

    /// Remove user from a group.
    pub async fn group_remove(&self, group_name: &str, user_email: &str) -> Result<()> {
        let url = endpoints::group_usermod(&self.config.connection.uri, group_name, user_email)?;
        self.send_request_raw(Method::DELETE, url, None::<()>).await?;
        Ok(())
    }

    /// Get all groups for on organization.
    pub async fn org_groups(&self, org_name: &str) -> Result<OrgGroupsResponse> {
        let url = endpoints::org_groups(&self.config.connection.uri, org_name)?;
        self.get(url).await
    }

    /// Create a new organization group.
    pub async fn org_create_group(&self, org_name: &str, group_name: &str) -> Result<()> {
        let url = endpoints::org_groups(&self.config.connection.uri, org_name)?;
        let body = ApiOrgGroup { name: group_name.into() };
        self.send_request_raw(Method::POST, url, Some(body)).await?;
        Ok(())
    }

    /// Delete an organization group.
    pub async fn org_delete_group(&self, org_name: &str, group_name: &str) -> Result<()> {
        let url = endpoints::org_groups_delete(&self.config.connection.uri, org_name, group_name)?;
        self.send_request_raw(Method::DELETE, url, None::<()>).await?;
        Ok(())
    }

    /// List a user's locksmith tokens.
    pub async fn list_tokens(&self) -> Result<Vec<UserToken>> {
        let url = endpoints::list_tokens(&self.config.connection.uri)?;
        self.get(url).await
    }

    /// Revoke a locksmith token.
    pub async fn revoke_token(&self, name: &str) -> Result<()> {
        let url = endpoints::revoke_token(&self.config.connection.uri)?;
        let body = RevokeTokenRequest { name };
        self.send_request_raw(Method::POST, url, Some(body)).await?;
        Ok(())
    }

    /// Get organizations the user is part of.
    pub async fn orgs(&self) -> Result<OrgsResponse> {
        let url = endpoints::orgs(&self.config.connection.uri)?;
        self.get(url).await
    }

    /// Get members of an organization.
    pub async fn org_members(&self, org: &str) -> Result<OrgMembersResponse> {
        let url = endpoints::org_members(&self.config.connection.uri, org)?;
        self.get(url).await
    }

    /// Add a member to an organization.
    pub async fn org_member_add(&self, org: &str, email: &str) -> Result<()> {
        let body = AddOrgUserRequest { email: email.into() };
        let url = endpoints::org_members(&self.config.connection.uri, org)?;
        self.send_request_raw(Method::POST, url, Some(body)).await?;
        Ok(())
    }

    /// Remove a member from an organization.
    pub async fn org_member_remove(&self, org: &str, email: &str) -> Result<()> {
        let url = endpoints::org_member_remove(&self.config.connection.uri, org, email)?;
        self.send_request_raw(Method::DELETE, url, None::<()>).await?;
        Ok(())
    }

    /// Get reachable vulnerabilities.
    #[cfg(feature = "vulnreach")]
    pub async fn vulnerabilities(&self, job: Job) -> Result<Vec<Vulnerability>> {
        let url = endpoints::vulnreach(&self.config.connection.uri)?;
        self.post(url, job).await
    }

    pub fn roles(&self) -> &HashSet<RealmRole> {
        &self.roles
    }

    pub fn config(&self) -> &Config {
        &self.config
    }
}

/// Phylum group types.
pub enum Group {
    Legacy(String),
    Org(OrgGroup),
}

impl Group {
    /// Create a group from an optional group name.
    pub fn try_new<O, G>(org: Option<O>, group: Option<G>) -> Option<Self>
    where
        O: Into<String>,
        G: Into<String>,
    {
        group.map(|group| match org {
            Some(org) => Self::Org(OrgGroup { org: org.into(), name: group.into() }),
            None => Self::Legacy(group.into()),
        })
    }

    /// Get the group's organization.
    pub fn org(&self) -> Option<&str> {
        match self {
            Self::Legacy(_) => None,
            Self::Org(OrgGroup { org, .. }) => Some(org.as_str()),
        }
    }

    /// Get the group's name.
    pub fn name(&self) -> &str {
        match self {
            Self::Legacy(name) => name.as_str(),
            Self::Org(OrgGroup { name, .. }) => name.as_str(),
        }
    }

    /// Format the group as a single string.
    pub fn combined_format(&self) -> Cow<'_, str> {
        match self {
            Self::Legacy(name) => Cow::Borrowed(name),
            Self::Org(OrgGroup { org, name }) => Cow::Owned(format!("{org}/{name}")),
        }
    }
}

/// Group under an organization.
pub struct OrgGroup {
    org: String,
    name: String,
}

/// Tests
#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use std::sync::{Arc, Mutex};

    use phylum_types::types::package::{PackageDescriptorAndLockfile, PackageType};
    use wiremock::http::HeaderName;
    use wiremock::matchers::{method, path, path_regex, query_param};
    use wiremock::{Mock, ResponseTemplate};

    use super::*;
    use crate::config::ConnectionInfo;
    use crate::test::mockito::*;
    use crate::types::{
        EvaluatedDependency, PolicyRejection, RejectionSource, RiskDomain, RiskLevel,
    };

    #[tokio::test]
    async fn create_client() -> Result<()> {
        let mock_server = build_mock_server().await;
        build_phylum_api(&mock_server).await?;
        Ok(())
    }

    #[tokio::test]
    async fn when_creating_unauthenticated_phylum_api_it_auths_itself() -> Result<()> {
        let mock_server = build_mock_server().await;
        let auth_info = build_unauthenticated_auth_info();

        let mut config = Config::default();
        config.connection = ConnectionInfo { uri: mock_server.uri() };
        config.auth_info = auth_info;

        let api = PhylumApi::new(config, None).await?;
        // After auth, auth_info should have a offline access token
        assert!(
            api.config().auth_info.offline_access().is_some(),
            "Offline access token was not set"
        );

        Ok(())
    }

    #[tokio::test]
    async fn when_submitting_a_request_phylum_api_includes_access_token() -> Result<()> {
        let mock_server = build_mock_server().await;

        let token_holder: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

        let responder_token_holder = token_holder.clone();

        Mock::given(method("POST"))
            .and(path("api/v0/data/jobs"))
            .respond_with_fn(move |request| {
                let mut guard = responder_token_holder.lock().unwrap();
                let auth_header = HeaderName::from_str("Authorization").unwrap();

                *guard = request.headers.get(&auth_header).map(|v| v.as_str().to_owned());

                ResponseTemplate::new(200)
                    .set_body_string(r#"{"job_id": "59482a54-423b-448d-8325-f171c9dc336b"}"#)
            })
            .mount(&mock_server)
            .await;

        let client = build_phylum_api(&mock_server).await?;

        let package_descriptor = PackageDescriptor {
            name: "react".to_string(),
            version: "16.13.1".to_string(),
            package_type: PackageType::Npm,
        };

        let pkg = AnalysisPackageDescriptor::PackageDescriptor(PackageDescriptorAndLockfile {
            package_descriptor,
            lockfile: Some("package-lock.json".to_owned()),
        });

        let project_id = ProjectId::new_v4();
        let label = Some("mylabel".to_string());
        client.submit_request(&[pkg], project_id, label, None).await?;

        // Request should have been submitted with a bearer token
        let bearer_token = token_holder.lock().unwrap().take();
        assert_eq!(Some(format!("Bearer {}", DUMMY_ACCESS_TOKEN)), bearer_token);

        Ok(())
    }

    #[tokio::test]
    async fn submit_request() -> Result<()> {
        let mock_server = build_mock_server().await;
        Mock::given(method("POST"))
            .and(path("api/v0/data/jobs"))
            .respond_with_fn(|_| {
                ResponseTemplate::new(200)
                    .set_body_string(r#"{"job_id": "59482a54-423b-448d-8325-f171c9dc336b"}"#)
            })
            .mount(&mock_server)
            .await;

        let client = build_phylum_api(&mock_server).await?;

        let package_descriptor = PackageDescriptor {
            name: "react".to_string(),
            version: "16.13.1".to_string(),
            package_type: PackageType::Npm,
        };

        let pkg = AnalysisPackageDescriptor::PackageDescriptor(PackageDescriptorAndLockfile {
            package_descriptor,
            lockfile: Some("package-lock.json".to_owned()),
        });

        let project_id = ProjectId::new_v4();
        let label = Some("mylabel".to_string());
        client.submit_request(&[pkg], project_id, label, None).await?;
        Ok(())
    }

    #[tokio::test]
    async fn get_status() -> Result<()> {
        let body = r#"
        {
            "count": 1,
            "jobs": [
                {
                    "date": "Mon, 17 May 2021 17:39:34 GMT",
                    "job_id": "e0ea0e13-f5f1-4142-85b8-7aa22bfb984f",
                    "label": "uncategorized",
                    "num_dependencies": 14,
                    "packages": [
                        {
                            "name": "ansi-red",
                            "type": "npm",
                            "version": "0.1.1"
                        }
                     ],
                    "msg": "Project met threshold requirements",
                    "pass": true,
                    "project": "test-project",
                    "total_jobs": 1,
                    "ecosystem": "npm"
                },
                {
                    "date": "Mon, 17 May 2021 17:39:34 GMT",
                    "job_id": "f8e8cb21-a4c0-4718-9cd2-8f631e95b951",
                    "label": "uncategorized",
                    "num_dependencies": 14,
                    "packages": [
                        {
                            "name": "ansi-red",
                            "type": "npm",
                            "version": "0.1.1"
                        }
                     ],
                    "msg": "Project met threshold requirements",
                    "pass": true,
                    "project": "test-project",
                    "total_jobs": 1,
                    "ecosystem": "npm"
                }

            ],
            "total_jobs": 1
        }"#;

        let mock_server = build_mock_server().await;
        Mock::given(method("GET"))
            .and(path("api/v0/data/jobs/"))
            .and(query_param("limit", "30"))
            .and(query_param("verbose", "1"))
            .respond_with_fn(move |_| ResponseTemplate::new(200).set_body_string(body))
            .mount(&mock_server)
            .await;

        let client = build_phylum_api(&mock_server).await?;
        client.get_status().await?;
        Ok(())
    }

    #[tokio::test]
    async fn get_package_details() -> Result<()> {
        let body = r#"
        {
          "status": "AlreadyProcessed",
          "data": {
            "id": "npm:@schematics/angular:9.1.9",
            "name": "@schematics/angular",
            "version": "9.1.9",
            "registry": "npm",
            "publishedDate": "1970-01-01T00:00:00+00:00",
            "latestVersion": null,
            "versions": [],
            "description": null,
            "license": null,
            "depSpecs": [],
            "dependencies": [],
            "downloadCount": 0,
            "riskScores": {
                "total": 1,
                "vulnerability": 1,
                "malicious_code": 1,
                "author": 1,
                "engineering": 1,
                "license": 1
            },
            "totalRiskScoreDynamics": null,
            "issuesDetails": [],
            "issues": [],
            "authors": [],
            "developerResponsiveness": {
                "open_issue_count": 167,
                "total_issue_count": 393,
                "open_issue_avg_duration": 980,
                "open_pull_request_count": 50,
                "total_pull_request_count": 476,
                "open_pull_request_avg_duration": 474
            },
            "complete": false
          }
        }
        "#;

        let mock_server = build_mock_server().await;
        Mock::given(method("POST"))
            .and(path("/api/v0/data/packages/submit"))
            .respond_with_fn(move |_| ResponseTemplate::new(200).set_body_string(body))
            .mount(&mock_server)
            .await;

        let client = build_phylum_api(&mock_server).await?;

        let pkg = PackageSpecifier {
            name: "@schematics/angular".to_string(),
            version: "9.1.9".to_string(),
            registry: "npm".to_string(),
        };
        client.submit_package(&pkg).await?;

        Ok(())
    }

    #[tokio::test]
    async fn get_job_status() -> Result<()> {
        let body = PolicyEvaluationResponse {
            is_failure: true,
            incomplete_count: 0,
            output: "output".into(),
            report: "report".into(),
        };
        let expected_body = body.clone();

        let mock_server = build_mock_server().await;
        Mock::given(method("POST"))
            .and(path_regex(r"^/api/v0/data/jobs/[-\dabcdef]+/policy/evaluate$".to_string()))
            .respond_with_fn(move |_| ResponseTemplate::new(200).set_body_json(body.clone()))
            .mount(&mock_server)
            .await;

        let client = build_phylum_api(&mock_server).await?;

        let job = JobId::from_str("59482a54-423b-448d-8325-f171c9dc336b").unwrap();
        let response = client.get_job_status(&job, []).await?;

        assert_eq!(response, expected_body);

        Ok(())
    }

    #[tokio::test]
    async fn get_job_status_raw() -> Result<()> {
        let body = PolicyEvaluationResponseRaw {
            is_failure: false,
            incomplete_packages_count: 3,
            help: "help".into(),
            dependencies: vec![EvaluatedDependency {
                purl: "purl".into(),
                registry: "registry".into(),
                name: "name".into(),
                version: "version".into(),
                rejections: vec![PolicyRejection {
                    title: "title".into(),
                    source: RejectionSource {
                        source_type: "source_type".into(),
                        tag: None,
                        domain: Some(RiskDomain::Vulnerabilities),
                        severity: Some(RiskLevel::Low),
                        description: None,
                        reason: None,
                    },
                    suppressed: false,
                }],
            }],
            job_link: Some("job_link".into()),
        };
        let expected_body = body.clone();

        let mock_server = build_mock_server().await;
        Mock::given(method("POST"))
            .and(path_regex(r"^/api/v0/data/jobs/[-\dabcdef]+/policy/evaluate/raw$".to_string()))
            .respond_with_fn(move |_| ResponseTemplate::new(200).set_body_json(body.clone()))
            .mount(&mock_server)
            .await;

        let client = build_phylum_api(&mock_server).await?;

        let job = JobId::from_str("59482a54-423b-448d-8325-f171c9dc336b").unwrap();
        let response = client.get_job_status_raw(&job, []).await?;

        assert_eq!(response, expected_body);

        Ok(())
    }

    #[tokio::test]
    async fn user_info() -> Result<()> {
        let body = r#"
        {
            "sub": "sub",
            "name": "John",
            "given_name": "John Doe",
            "family_name": "Doe",
            "preferred_username": "johnny",
            "email": "john-doe@example.org",
            "email_verified": true
        }"#;

        let mock_server = build_mock_server().await;
        Mock::given(method("GET"))
            .and(path(USER_URI))
            .respond_with_fn(move |_| ResponseTemplate::new(200).set_body_string(body))
            .mount(&mock_server)
            .await;

        let client = build_phylum_api(&mock_server).await?;

        let user = client.user_info().await?;

        assert_eq!(user.email, "john-doe@example.org");

        Ok(())
    }
}
