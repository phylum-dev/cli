use std::time::Duration;

use anyhow::{anyhow, Context};
use phylum_types::types::auth::TokenResponse;
use phylum_types::types::common::{JobId, ProjectId};
use phylum_types::types::group::{
    CreateGroupRequest, CreateGroupResponse, ListGroupMembersResponse, ListUserGroupsResponse,
};
use phylum_types::types::job::{
    AllJobsStatusResponse, JobStatusResponse, SubmitPackageRequest, SubmitPackageResponse,
};
use phylum_types::types::package::{
    PackageDescriptor, PackageSpecifier, PackageStatus, PackageStatusExtended,
    PackageSubmitResponse, PackageType,
};
use phylum_types::types::project::{
    CreateProjectRequest, CreateProjectResponse, ProjectSummaryResponse,
};
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{Client, IntoUrl, Method, StatusCode};
use serde::de::{DeserializeOwned, IgnoredAny};
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

use self::endpoints::BaseUriError;
use crate::app::USER_AGENT;
use crate::auth::{
    fetch_oidc_server_settings, handle_auth_flow, handle_refresh_tokens, AuthAction, UserInfo,
};
use crate::config::{AuthInfo, Config};
use crate::types::{HistoryJob, PingResponse};

pub mod endpoints;

type Result<T> = std::result::Result<T, PhylumApiError>;

pub struct PhylumApi {
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
}

impl PhylumApi {
    /// Create a phylum API client using the given Auth configuration, api url
    /// and request timeout. If in the process of creating the client,
    /// credentials must be obtained, the auth_info struct will be updated
    /// with the new information. It is the duty of the calling code to save
    /// any changes
    pub async fn new(mut config: Config, request_timeout: Option<u64>) -> Result<Self> {
        // Do we have a refresh token?
        let tokens: TokenResponse = match &config.auth_info.offline_access() {
            Some(refresh_token) => {
                handle_refresh_tokens(refresh_token, config.ignore_certs(), &config.connection.uri)
                    .await
                    .context("Token refresh failed")?
            },
            None => {
                handle_auth_flow(AuthAction::Login, config.ignore_certs(), &config.connection.uri)
                    .await
                    .context("User login failed")?
            },
        };

        config.auth_info.set_offline_access(tokens.refresh_token.clone());

        let mut headers = HeaderMap::new();
        // the cli runs a command or a few short commands then exits, so we do
        // not need to worry about refreshing the access token. We just set it
        // here and be done.
        headers.insert(
            "Authorization",
            HeaderValue::from_str(&format!("Bearer {}", tokens.access_token)).unwrap(),
        );
        headers.insert("Accept", HeaderValue::from_str("application/json").unwrap());

        let client = Client::builder()
            .user_agent(USER_AGENT.as_str())
            .timeout(Duration::from_secs(request_timeout.unwrap_or(std::u64::MAX)))
            .danger_accept_invalid_certs(config.ignore_certs())
            .default_headers(headers)
            .build()?;

        Ok(Self { config, client })
    }

    /// update auth info by forcing the login flow, using the given Auth
    /// configuration. The auth_info struct will be updated with the new
    /// credentials. It is the duty of the calling code to save any changes
    pub async fn login(
        mut auth_info: AuthInfo,
        ignore_certs: bool,
        api_uri: &str,
        reauth: bool,
    ) -> Result<AuthInfo> {
        let action = if reauth { AuthAction::Reauth } else { AuthAction::Login };
        let tokens = handle_auth_flow(action, ignore_certs, api_uri).await?;
        auth_info.set_offline_access(tokens.refresh_token);
        Ok(auth_info)
    }

    /// update auth info by forcing the registration flow, using the given Auth
    /// configuration. The auth_info struct will be updated with the new
    /// credentials. It is the duty of the calling code to save any changes
    pub async fn register(
        mut auth_info: AuthInfo,
        ignore_certs: bool,
        api_uri: &str,
    ) -> Result<AuthInfo> {
        let tokens = handle_auth_flow(AuthAction::Register, ignore_certs, api_uri).await?;
        auth_info.set_offline_access(tokens.refresh_token);
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
        let oidc_settings =
            fetch_oidc_server_settings(self.config.ignore_certs(), &self.config.connection.uri)
                .await?;
        self.get(oidc_settings.userinfo_endpoint).await
    }

    /// Create a new project
    pub async fn create_project(&self, name: &str, group: Option<&str>) -> Result<ProjectId> {
        let response: CreateProjectResponse = self
            .post(
                endpoints::post_create_project(&self.config.connection.uri)?,
                CreateProjectRequest { name: name.to_owned(), group_name: group.map(String::from) },
            )
            .await?;
        Ok(response.id)
    }

    /// Delete a project
    pub async fn delete_project(&self, project_id: ProjectId) -> Result<()> {
        let _: IgnoredAny = self
            .delete(endpoints::delete_project(
                &self.config.connection.uri,
                &format!("{project_id}"),
            )?)
            .await?;
        Ok(())
    }

    /// Get a list of projects
    pub async fn get_projects(&self, group: Option<&str>) -> Result<Vec<ProjectSummaryResponse>> {
        let uri = match group {
            Some(group) => endpoints::group_project_summary(&self.config.connection.uri, group)?,
            None => endpoints::get_project_summary(&self.config.connection.uri)?,
        };

        self.get(uri).await
    }

    /// Submit a new request to the system
    pub async fn submit_request(
        &self,
        package_list: &[PackageDescriptor],
        project: ProjectId,
        label: Option<String>,
        group_name: Option<String>,
    ) -> Result<JobId> {
        #[allow(deprecated)]
        let req = SubmitPackageRequest {
            // This package_type is ignored by the API, but it still validates it, so we have to put
            // something here.
            package_type: Some(PackageType::Npm),
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

    /// Get the status of a previously submitted job
    pub async fn get_job_status(&self, job_id: &JobId) -> Result<JobStatusResponse<PackageStatus>> {
        self.get(endpoints::get_job_status(&self.config.connection.uri, job_id, false)?).await
    }

    /// Get the status of a previously submitted job (verbose output)
    pub async fn get_job_status_ext(
        &self,
        job_id: &JobId,
    ) -> Result<JobStatusResponse<PackageStatusExtended>> {
        self.get(endpoints::get_job_status(&self.config.connection.uri, job_id, true)?).await
    }

    /// Get the status of all jobs
    pub async fn get_status(&self) -> Result<AllJobsStatusResponse> {
        self.get(endpoints::get_all_jobs_status(&self.config.connection.uri, 30)?).await
    }

    /// Get project's job history.
    pub async fn get_project_history(
        &self,
        project_name: &str,
        group_name: Option<&str>,
    ) -> Result<Vec<HistoryJob>> {
        let project_id = self.get_project_id(project_name, group_name).await?.to_string();

        let url = match group_name {
            Some(group_name) => endpoints::get_group_project_history(
                &self.config.connection.uri,
                &project_id,
                group_name,
            )?,
            None => endpoints::get_project_history(&self.config.connection.uri, &project_id)?,
        };

        self.get(url).await
    }

    /// Resolve a Project Name to a Project ID
    pub async fn get_project_id(
        &self,
        project_name: &str,
        group_name: Option<&str>,
    ) -> Result<ProjectId> {
        let projects = self.get_projects(group_name).await?;

        projects
            .iter()
            .find(|project| project.name == project_name)
            .ok_or_else(|| anyhow!("No project found with name {:?}", project_name).into())
            .map(|project| project.id)
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

    /// Change group ownership.
    pub async fn group_set_owner(&self, group_name: &str, new_owner_email: &str) -> Result<()> {
        let url = endpoints::set_owner(&self.config.connection.uri, group_name, new_owner_email)?;
        self.send_request_raw(Method::PUT, url, None::<()>).await?;
        Ok(())
    }

    pub fn config(&self) -> &Config {
        &self.config
    }
}

/// Tests
#[cfg(test)]
mod tests {

    use std::str::FromStr;
    use std::sync::{Arc, Mutex};

    use wiremock::http::HeaderName;
    use wiremock::matchers::{method, path, path_regex, query_param};
    use wiremock::{Mock, ResponseTemplate};

    use super::*;
    use crate::config::ConnectionInfo;
    use crate::test::mockito::*;

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

        let pkg = PackageDescriptor {
            name: "react".to_string(),
            version: "16.13.1".to_string(),
            package_type: PackageType::Npm,
        };
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

        let pkg = PackageDescriptor {
            name: "react".to_string(),
            version: "16.13.1".to_string(),
            package_type: PackageType::Npm,
        };
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
                    "action": "warn",
                    "project": "test-project",
                    "total_jobs": 1,
                    "score": 1.0,
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
                    "action": "break",
                    "project": "test-project",
                    "total_jobs": 1,
                    "score": 1.0,
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
            "issueImpacts": {
                "low": 0,
                "medium": 0,
                "high": 0,
                "critical": 0
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
        let body = r#"
        {
            "job_id": "59482a54-423b-448d-8325-f171c9dc336b",
            "user_id": "86bb664a-5331-489b-8901-f052f155ec79",
            "ecosystem": "npm",
            "user_email": "foo@bar.com",
            "thresholds": {
                "author": 0.4,
                "engineering": 0.2,
                "license": 0.5,
                "malicious": 0.42,
                "vulnerability": 0.8,
                "total": 0.6
            },
            "created_at": 1603311564,
            "status": "complete",
            "score": 1.0,
            "last_updated": 1603311780,
            "project": "86bb664a-5331-489b-8901-f052f155ec79",
            "project_name": "some_project",
            "label": "some_label",
            "msg": "Project met threshold requirements",
            "pass": true,
            "action": "none",
            "packages": [
                {
                "name": "foo",
                "version": "1.0.0",
                "type": "npm",
                "status": "complete",
                "last_updated": 1603311564,
                "license": null,
                "num_dependencies": 2,
                "num_vulnerabilities": 4,
                "package_score": 0.85
                }]}"#;

        let mock_server = build_mock_server().await;
        Mock::given(method("GET"))
            .and(path_regex(r"^/api/v0/data/jobs/[-\dabcdef]+$".to_string()))
            .respond_with_fn(move |_| ResponseTemplate::new(200).set_body_string(body))
            .mount(&mock_server)
            .await;

        let client = build_phylum_api(&mock_server).await?;

        let job = JobId::from_str("59482a54-423b-448d-8325-f171c9dc336b").unwrap();
        client.get_job_status(&job).await?;

        Ok(())
    }

    #[tokio::test]
    async fn get_job_status_ext() -> Result<()> {
        let body = r#"
        {
            "job_id": "59482a54-423b-448d-8325-f171c9dc336b",
            "user_id": "86bb664a-5331-489b-8901-f052f155ec79",
            "ecosystem": "npm",
            "project": "86bb664a-5331-489b-8901-f052f155ec79",
            "project_name": "some project",
            "user_email": "foo@bar.com",
            "thresholds": {
                "author": 0.4,
                "engineering": 0.2,
                "license": 0.5,
                "malicious": 0.42,
                "vulnerability": 0.8,
                "total": 0.6
            },
            "created_at": 1603311564,
            "score": 1.0,
            "label": "",
            "status": "incomplete",
            "last_updated": 1603311864,
            "msg": "Project met threshold requirements",
            "pass": true,
            "action": "warn",
            "packages": [
                {
                    "name": "foo",
                    "version": "1.0.0",
                    "type": "npm",
                    "last_updated": 1603311864,
                    "license": null,
                    "num_dependencies": 2,
                    "num_vulnerabilities": 7,
                    "package_score": 0.3,
                    "status": "incomplete",
                    "issues": [
                        {
                            "title": "Commercial license risk in xmlrpc@0.3.0",
                            "description": "license is medium risk",
                            "severity": "medium",
                            "domain": "license"
                        }
                    ],
                    "riskVectors": {
                        "author": 0.9,
                        "engineering": 0.42,
                        "license": 1.0,
                        "malicious_code": 1.0,
                        "vulnerability": 1.0
                    },
                    "dependencies": {
                        "bar": "^2.3.4",
                        "baz": "<9.8.7"
                    }
                }
            ]
        }"#;

        let mock_server = build_mock_server().await;
        Mock::given(method("GET"))
            .and(path_regex(r"^/api/v0/data/jobs/[-\dabcdef]+".to_string()))
            .and(query_param("verbose", "True"))
            .respond_with_fn(move |_| ResponseTemplate::new(200).set_body_string(body))
            .mount(&mock_server)
            .await;

        let client = build_phylum_api(&mock_server).await?;

        let job = JobId::from_str("59482a54-423b-448d-8325-f171c9dc336b").unwrap();
        client.get_job_status_ext(&job).await?;

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
