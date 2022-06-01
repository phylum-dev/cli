use std::time::Duration;

use anyhow::anyhow;
use anyhow::Context;
use phylum_types::types::auth::*;
use phylum_types::types::common::*;
use phylum_types::types::group::{CreateGroupRequest, CreateGroupResponse, ListUserGroupsResponse};
use phylum_types::types::job::*;
use phylum_types::types::package::*;
use phylum_types::types::project::CreateProjectRequest;
use phylum_types::types::project::CreateProjectResponse;
use phylum_types::types::project::ProjectDetailsResponse;
use phylum_types::types::project::ProjectSummaryResponse;
use phylum_types::types::user_settings::*;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{Client, Method, StatusCode};
use serde::de::DeserializeOwned;
use serde::Serialize;
use thiserror::Error as ThisError;

pub mod endpoints;

use crate::auth::fetch_oidc_server_settings;
use crate::auth::handle_auth_flow;
use crate::auth::handle_refresh_tokens;
use crate::auth::{AuthAction, UserInfo};
use crate::config::AuthInfo;
use crate::types::PingResponse;

type Result<T> = std::result::Result<T, PhylumApiError>;

pub struct PhylumApi {
    client: Client,
    api_uri: String,
    ignore_certs: bool,
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
    Other(#[from] anyhow::Error),
}

impl PhylumApiError {
    pub fn status(&self) -> Option<StatusCode> {
        match self {
            PhylumApiError::ReqwestError { source } => source.status(),
            PhylumApiError::Other(_) => None,
        }
    }
}

impl PhylumApi {
    async fn get<T: DeserializeOwned>(&self, path: String) -> Result<T> {
        self.send_request::<_, ()>(Method::GET, path, None).await
    }

    async fn put<T: DeserializeOwned, S: serde::Serialize>(&self, path: String, s: S) -> Result<T> {
        self.send_request(Method::PUT, path, Some(s)).await
    }

    async fn post<T: serde::de::DeserializeOwned, S: serde::Serialize>(
        &self,
        path: String,
        s: S,
    ) -> Result<T> {
        self.send_request(Method::POST, path, Some(s)).await
    }

    async fn send_request<T: DeserializeOwned, B: Serialize>(
        &self,
        method: Method,
        path: String,
        body: Option<B>,
    ) -> Result<T> {
        let mut request = self.client.request(method, path);
        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request.send().await?;
        let success = response.status().is_success();
        let body = response.text().await?;

        if !success {
            return Err(anyhow!(body).into());
        }

        serde_json::from_str::<T>(&body).map_err(|e| PhylumApiError::Other(e.into()))
    }
}

impl PhylumApi {
    /// Create a phylum API client using the given Auth configuration, api url and
    /// request timeout. If in the process of creating the client, credentials
    /// must be obtained, the auth_info struct will be updated with the new
    /// information. It is the duty of the calling code to save any changes
    pub async fn new(
        auth_info: &mut AuthInfo,
        api_uri: &str,
        request_timeout: Option<u64>,
        ignore_certs: bool,
    ) -> Result<Self> {
        // Do we have a refresh token?
        let tokens: TokenResponse = match &auth_info.offline_access {
            Some(refresh_token) => {
                handle_refresh_tokens(refresh_token, ignore_certs, api_uri).await?
            }
            None => handle_auth_flow(&AuthAction::Login, ignore_certs, api_uri).await?,
        };

        auth_info.offline_access = Some(tokens.refresh_token.clone());

        let version = env!("CARGO_PKG_VERSION");
        let mut headers = HeaderMap::new();
        // the cli runs a command or a few short commands then exits, so we do
        // not need to worry about refreshing the access token. We just set it
        // here and be done.
        headers.insert(
            "Authorization",
            HeaderValue::from_str(&format!("Bearer {}", tokens.access_token)).unwrap(),
        );
        headers.insert("Accept", HeaderValue::from_str("application/json").unwrap());
        headers.insert("version", HeaderValue::from_str(version).unwrap());

        let client = Client::builder()
            .timeout(Duration::from_secs(
                request_timeout.unwrap_or(std::u64::MAX),
            ))
            .danger_accept_invalid_certs(ignore_certs)
            .default_headers(headers)
            .build()?;

        Ok(Self {
            client,
            api_uri: api_uri.to_string(),
            ignore_certs,
        })
    }

    /// update auth info by forcing the login flow, using the given Auth
    /// configuration. The auth_info struct will be updated with the new
    /// credentials. It is the duty of the calling code to save any changes
    pub async fn login(
        mut auth_info: AuthInfo,
        ignore_certs: bool,
        api_uri: &str,
    ) -> Result<AuthInfo> {
        let tokens = handle_auth_flow(&AuthAction::Login, ignore_certs, api_uri).await?;
        auth_info.offline_access = Some(tokens.refresh_token);
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
        let tokens = handle_auth_flow(&AuthAction::Register, ignore_certs, api_uri).await?;
        auth_info.offline_access = Some(tokens.refresh_token);
        Ok(auth_info)
    }

    /// Ping the system and verify it's up
    pub async fn ping(&self) -> Result<String> {
        Ok(self
            .get::<PingResponse>(endpoints::get_ping(&self.api_uri))
            .await?
            .response)
    }

    /// Get information about the authenticated user
    pub async fn user_info(&self) -> Result<UserInfo> {
        let oidc_settings = fetch_oidc_server_settings(self.ignore_certs, &self.api_uri).await?;
        self.get(oidc_settings.userinfo_endpoint.into()).await
    }

    /// Create a new project
    pub async fn create_project(&self, name: &str, group: Option<&str>) -> Result<ProjectId> {
        Ok(self
            .put::<CreateProjectResponse, _>(
                endpoints::put_create_project(&self.api_uri),
                CreateProjectRequest {
                    name: name.to_owned(),
                    group_name: group.map(String::from),
                },
            )
            .await?
            .id)
    }

    /// Get a list of projects
    pub async fn get_projects(&self, group: Option<&str>) -> Result<Vec<ProjectSummaryResponse>> {
        let uri = match group {
            Some(group) => endpoints::group_project_summary(&self.api_uri, group),
            None => endpoints::get_project_summary(&self.api_uri),
        };

        self.get(uri).await
    }

    /// Get user settings
    pub async fn get_user_settings(&self) -> Result<UserSettings> {
        self.get(endpoints::get_user_settings(&self.api_uri)).await
    }

    /// Put updated user settings
    pub async fn put_user_settings(&self, settings: &UserSettings) -> Result<bool> {
        self.put::<UserSettings, _>(endpoints::put_user_settings(&self.api_uri), &settings)
            .await?;
        Ok(true)
    }

    /// Submit a new request to the system
    pub async fn submit_request(
        &self,
        req_type: &PackageType,
        package_list: &[PackageDescriptor],
        is_user: bool,
        project: ProjectId,
        label: Option<String>,
        group_name: Option<String>,
    ) -> Result<JobId> {
        let req = SubmitPackageRequest {
            package_type: req_type.to_owned(),
            packages: package_list.to_vec(),
            is_user,
            project,
            label: label.unwrap_or_else(|| "uncategorized".to_string()),
            group_name,
        };
        log::debug!("==> Sending package submission: {:?}", req);
        let resp: SubmitPackageResponse = self
            .put(endpoints::put_submit_package(&self.api_uri), req)
            .await?;
        Ok(resp.job_id)
    }

    /// Get the status of a previously submitted job
    pub async fn get_job_status(&self, job_id: &JobId) -> Result<JobStatusResponse<PackageStatus>> {
        self.get(endpoints::get_job_status(&self.api_uri, job_id, false))
            .await
    }

    /// Get the status of a previously submitted job (verbose output)
    pub async fn get_job_status_ext(
        &self,
        job_id: &JobId,
    ) -> Result<JobStatusResponse<PackageStatusExtended>> {
        self.get(endpoints::get_job_status(&self.api_uri, job_id, true))
            .await
    }

    /// Get the status of all jobs
    pub async fn get_status(&self) -> Result<AllJobsStatusResponse> {
        self.get(endpoints::get_all_jobs_status(&self.api_uri, 30))
            .await
    }

    /// Get the details of a specific project
    pub async fn get_project_details(&self, project_name: &str) -> Result<ProjectDetailsResponse> {
        self.get(endpoints::get_project_details(&self.api_uri, project_name))
            .await
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
            .and_then(|project| {
                project
                    .id
                    .parse()
                    .context("Invalid Project ID")
                    .map_err(PhylumApiError::from)
            })
    }

    /// Get package details
    pub async fn get_package_details(&self, pkg: &PackageDescriptor) -> Result<Package> {
        self.get(endpoints::get_package_status(&self.api_uri, pkg))
            .await
    }

    /// Get all groups the user is part of.
    pub async fn get_groups_list(&self) -> Result<ListUserGroupsResponse> {
        self.get(endpoints::group_list(&self.api_uri)).await
    }

    /// Get all groups the user is part of.
    pub async fn create_group(&self, group_name: &str) -> Result<CreateGroupResponse> {
        let group = CreateGroupRequest {
            group_name: group_name.into(),
        };
        self.post(endpoints::group_create(&self.api_uri), group)
            .await
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

    use crate::test::mockito::*;

    use super::*;
    #[tokio::test]
    async fn create_client() -> Result<()> {
        let mock_server = build_mock_server().await;
        build_phylum_api(&mock_server).await?;
        Ok(())
    }

    #[tokio::test]
    async fn when_creating_unauthenticated_phylum_api_it_auths_itself() -> Result<()> {
        let mock_server = build_mock_server().await;
        let mut auth_info = build_unauthenticated_auth_info();
        PhylumApi::new(&mut auth_info, mock_server.uri().as_str(), None, false).await?;
        // After auth, auth_info should have a offline access token
        assert!(
            auth_info.offline_access.is_some(),
            "Offline access token was not set"
        );

        Ok(())
    }

    #[tokio::test]
    async fn when_submitting_a_request_phylum_api_includes_access_token() -> Result<()> {
        let mock_server = build_mock_server().await;

        let token_holder: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

        let responder_token_holder = token_holder.clone();

        Mock::given(method("PUT"))
            .and(path("api/v0/job"))
            .respond_with_fn(move |request| {
                let mut guard = responder_token_holder.lock().unwrap();
                let auth_header = HeaderName::from_str("Authorization").unwrap();

                *guard = request
                    .headers
                    .get(&auth_header)
                    .map(|v| v.as_str().to_owned());

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
        client
            .submit_request(&PackageType::Npm, &[pkg], true, project_id, label, None)
            .await?;

        // Request should have been submitted with a bearer token
        let bearer_token = token_holder.lock().unwrap().take();
        assert_eq!(Some(format!("Bearer {}", DUMMY_ACCESS_TOKEN)), bearer_token);

        Ok(())
    }

    #[tokio::test]
    async fn submit_request() -> Result<()> {
        let mock_server = build_mock_server().await;
        Mock::given(method("PUT"))
            .and(path("api/v0/job"))
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
        client
            .submit_request(&PackageType::Npm, &[pkg], true, project_id, label, None)
            .await?;
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
            .and(path("api/v0/job/"))
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
          "id": "npm:@schematics~angular:9.1.9",
          "name": "@schematics~angular",
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
        "#;

        let mock_server = build_mock_server().await;
        Mock::given(method("GET"))
            .and(path("/api/v0/data/packages/npm/@schematics~angular/9.1.9"))
            .respond_with_fn(move |_| ResponseTemplate::new(200).set_body_string(body))
            .mount(&mock_server)
            .await;

        let client = build_phylum_api(&mock_server).await?;

        let pkg = PackageDescriptor {
            name: "@schematics/angular".to_string(),
            version: "9.1.9".to_string(),
            package_type: PackageType::Npm,
        };
        client.get_package_details(&pkg).await?;

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
            .and(path_regex(r"^/api/v0/job/[-\dabcdef]+$".to_string()))
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
            .and(path_regex(r"^/api/v0/job/[-\dabcdef]+".to_string()))
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
