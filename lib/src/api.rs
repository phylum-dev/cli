use std::time::Duration;

use thiserror::Error as ThisError;

use crate::auth::*;
use crate::config::AuthInfo;
use crate::restson::{Error as RestsonError, RestClient};
use crate::types::*;

pub struct PhylumApi {
    client: RestClient,
}

/// Phylum Api Error type
#[derive(ThisError, Debug)]
pub enum PhylumApiError {
    #[error("Error invoking restson endpoint")]
    RestsonError {
        #[from]
        source: RestsonError,
    },
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl PhylumApi {
    /// Create a phylum API client using the given Auth configuration, api url and
    /// request timeout. If in the process of creating the client, credentials
    /// must be obtained, the auth_info struct will be updated with the new
    /// information. It is the duty of the calling code to save any changes
    pub async fn new(
        auth_info: &mut AuthInfo,
        api_url: &str,
        request_timeout: Option<u64>,
    ) -> Result<Self, PhylumApiError> {
        // Do we have a refresh token?
        let tokens: TokenResponse = match &auth_info.offline_access {
            Some(refresh_token) => handle_refresh_tokens(auth_info, refresh_token).await?,
            None => handle_auth_flow(&AuthAction::Login, auth_info).await?,
        };

        auth_info.offline_access = Some(tokens.refresh_token.clone());

        let timeout = request_timeout.unwrap_or(30);
        log::debug!("Setting request timeout to {} seconds", timeout);

        let mut client = RestClient::builder()
            .timeout(Duration::from_secs(timeout))
            .build(api_url)?;

        // the cli runs a command or a few short commands then exits, so we do
        // not need to worry about refreshing the access token. We just set it
        // here and be done.
        client.set_jwt_auth((&tokens.access_token).into())?;

        let yml = clap::load_yaml!("bin/.conf/cli.yaml");
        let version = yml["version"].as_str().unwrap_or("");
        client.set_header("version", version)?;

        Ok(Self { client })
    }

    /// update auth info by forcing the login flow, using the given Auth
    /// configuration. The auth_info struct will be updated with the new
    /// credentials. It is the duty of the calling code to save any changes
    pub async fn login(mut auth_info: AuthInfo) -> Result<AuthInfo, PhylumApiError> {
        let tokens = handle_auth_flow(&AuthAction::Login, &auth_info).await?;
        auth_info.offline_access = Some(tokens.refresh_token);
        Ok(auth_info)
    }

    /// update auth info by forcing the registration flow, using the given Auth
    /// configuration. The auth_info struct will be updated with the new
    /// credentials. It is the duty of the calling code to save any changes
    pub async fn register(mut auth_info: AuthInfo) -> Result<AuthInfo, PhylumApiError> {
        let tokens = handle_auth_flow(&AuthAction::Register, &auth_info).await?;
        auth_info.offline_access = Some(tokens.refresh_token);
        Ok(auth_info)
    }

    /// Ping the system and verify it's up
    pub async fn ping(&mut self) -> Result<String, RestsonError> {
        let resp: PingResponse = self.client.get(()).await?;
        Ok(resp.msg)
    }

    /// Check auth status of the current user
    pub async fn auth_status(&mut self) -> Result<bool, RestsonError> {
        let resp: AuthStatusResponse = self.client.get(()).await?;
        Ok(resp.authenticated)
    }

    /// Create a new project
    pub async fn create_project(&mut self, name: &str) -> Result<ProjectId, RestsonError> {
        let req = ProjectCreateRequest {
            name: name.to_string(),
        };
        let resp: ProjectCreateResponse = self.client.put_capture((), &req).await?;
        Ok(resp.id)
    }

    /// Get a list of projects
    pub async fn get_projects(&mut self) -> Result<Vec<ProjectGetRequest>, RestsonError> {
        let resp: Vec<ProjectGetRequest> = self.client.get(()).await?;
        Ok(resp)
    }

    /// Get user settings
    pub async fn get_user_settings(&mut self) -> Result<UserSettings, RestsonError> {
        let resp: UserSettings = self.client.get(()).await?;
        Ok(resp)
    }

    /// Put updated user settings
    pub async fn put_user_settings(
        &mut self,
        settings: &UserSettings,
    ) -> Result<bool, RestsonError> {
        let _resp: UserSettings = self.client.put_capture((), settings).await?;
        Ok(true)
    }

    /// Submit a new request to the system
    pub async fn submit_request(
        &mut self,
        req_type: &PackageType,
        package_list: &[PackageDescriptor],
        is_user: bool,
        project: ProjectId,
        label: Option<String>,
    ) -> Result<JobId, RestsonError> {
        let req = PackageRequest {
            r#type: req_type.to_owned(),
            packages: package_list.to_vec(),
            is_user,
            project,
            label: label.unwrap_or_else(|| "uncategorized".to_string()),
        };
        log::debug!("==> Sending package submission: {:?}", req);
        let resp: PackageSubmissionResponse = self.client.put_capture((), &req).await?;
        Ok(resp.job_id)
    }

    /// Get the status of a previously submitted job
    pub async fn get_job_status(
        &mut self,
        job_id: &JobId,
    ) -> Result<RequestStatusResponse<PackageStatus>, RestsonError> {
        let resp: RequestStatusResponse<PackageStatus> = self.client.get(job_id.to_owned()).await?;
        Ok(resp)
    }

    /// Get the status of a previously submitted job (verbose output)
    pub async fn get_job_status_ext(
        &mut self,
        job_id: &JobId,
    ) -> Result<RequestStatusResponse<PackageStatusExtended>, RestsonError> {
        let resp: RequestStatusResponse<PackageStatusExtended> =
            self.client.get(job_id.to_owned()).await?;
        Ok(resp)
    }

    /// Get the status of all jobs
    pub async fn get_status(&mut self) -> Result<AllJobsStatusResponse, RestsonError> {
        let resp: AllJobsStatusResponse = self.client.get(30).await?;
        Ok(resp)
    }

    /// Get the details of a specific project
    pub async fn get_project_details(
        &mut self,
        project_name: &str,
    ) -> Result<ProjectGetDetailsRequest, RestsonError> {
        let resp: ProjectGetDetailsRequest = self.client.get(project_name).await?;
        Ok(resp)
    }

    /// Get package details
    pub async fn get_package_details(
        &mut self,
        pkg: &PackageDescriptor,
    ) -> Result<PackageStatusExtended, RestsonError> {
        let resp: PackageStatusExtended = self.client.get(pkg.to_owned()).await?;
        Ok(resp)
    }

    /// Cancel a job currently in progress
    pub async fn cancel(&mut self, job_id: &JobId) -> Result<CancelRequestResponse, RestsonError> {
        let resp: CancelRequestResponse = self.client.delete_capture(job_id.to_owned()).await?;
        Ok(resp)
    }
}

/// Tests
#[cfg(test)]
mod tests {

    use std::str::FromStr;
    use uuid::Uuid;
    use wiremock::matchers::{method, path, path_regex, query_param};
    use wiremock::{Mock, ResponseTemplate};

    use crate::test::mockito::*;

    use super::*;
    #[tokio::test]
    async fn create_client() -> Result<(), PhylumApiError> {
        let mock_server = build_mock_server().await;
        build_phylum_api(&mock_server).await?;
        Ok(())
    }

    #[tokio::test]
    async fn submit_request() -> Result<(), PhylumApiError> {
        let mock_server = build_mock_server().await;
        Mock::given(method("PUT"))
            .and(path("api/v0/job"))
            .respond_with_fn(|_| {
                ResponseTemplate::new(200)
                    .set_body_string(r#"{"job_id": "59482a54-423b-448d-8325-f171c9dc336b"}"#)
            })
            .mount(&mock_server)
            .await;

        let mut client = build_phylum_api(&mock_server).await?;

        let pkg = PackageDescriptor {
            name: "react".to_string(),
            version: "16.13.1".to_string(),
            r#type: PackageType::Npm,
        };
        let project_id = Uuid::new_v4();
        let label = Some("mylabel".to_string());
        client
            .submit_request(&PackageType::Npm, &[pkg], true, project_id, label)
            .await?;
        Ok(())
    }

    #[tokio::test]
    async fn get_status() -> Result<(), PhylumApiError> {
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

        let mut client = build_phylum_api(&mock_server).await?;
        client.get_status().await?;
        Ok(())
    }

    #[tokio::test]
    async fn get_package_details() -> Result<(), PhylumApiError> {
        let body = r#"
        {
            "name": "@schematics/angular",
            "version": "9.1.9",
            "type": "npm",
            "last_updated": 1611962723183,
            "license": "MIT",
            "package_score": 1.0,
            "num_dependencies": 2,
            "num_vulnerabilities": 4,
            "msg": "Project met threshold requirements",
            "pass": true,
            "action": "warn",
            "status": "complete",
            "vulnerabilities": [],
            "riskVectors": {
                "author": 0.90,
                "engineering": 0.42,
                "license": 1.0,
                "malicious_code": 1.0,
                "vulnerability": 1.0
            },
            "issues": [
                {
                "title": "Commercial license risk in xmlrpc@0.3.0",
                "description": "license is medium risk",
                "risk_level": "medium",
                "risk_domain": "LicenseRisk",
                "pkg_name": "xmlrpc",
                "pkg_version": "0.3.0",
                "score": 0.7
                }
            ],
            "heuristics": {
                "something": {
                    "description": "do stuff",
                    "score": 3.14,
                    "domain": "AuthorRisk",
                    "risk_level": "medium"
                }
            },
            "dependencies": []
          }
        "#;

        let mock_server = build_mock_server().await;
        Mock::given(method("GET"))
            .and(path("/api/v0/job/packages/npm/@schematics~angular/9.1.9"))
            .respond_with_fn(move |_| ResponseTemplate::new(200).set_body_string(body))
            .mount(&mock_server)
            .await;

        let mut client = build_phylum_api(&mock_server).await?;

        let pkg = PackageDescriptor {
            name: "@schematics/angular".to_string(),
            version: "9.1.9".to_string(),
            r#type: PackageType::Npm,
        };
        client.get_package_details(&pkg).await?;

        Ok(())
    }

    #[tokio::test]
    async fn get_job_status() -> Result<(), PhylumApiError> {
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

        let mut client = build_phylum_api(&mock_server).await?;

        let job = JobId::from_str("59482a54-423b-448d-8325-f171c9dc336b").unwrap();
        client.get_job_status(&job).await?;

        Ok(())
    }

    #[tokio::test]
    async fn get_job_status_ext() -> Result<(), PhylumApiError> {
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
                "vulnerabilities": [],
                "issues": [
                    {
                    "title": "Commercial license risk in xmlrpc@0.3.0",
                    "description": "license is medium risk",
                    "risk_level": "medium",
                    "risk_domain": "LicenseRisk",
                    "pkg_name": "xmlrpc",
                    "pkg_version": "0.3.0",
                    "score": 0.7
                    }
                ],
                "heuristics": {
                    "something": {
                        "description": "do stuff",
                        "score": 3.14,
                        "domain": "EngineeringRisk",
                        "risk_level": "critical"
                    }
                },
                "riskVectors": {
                    "author": 0.90,
                    "engineering": 0.42,
                    "license": 1.0,
                    "malicious_code": 1.0,
                    "vulnerability": 1.0
                },
                "dependencies": [
                    {
                    "name": "bar",
                    "version": "2.3.4",
                    "type": "npm",
                    "last_updated": 1603311564,
                    "license": null,
                    "package_score": 60.0,
                    "status": "incomplete",
                    "vulnerabilities": [],
                    "heuristics": []
                    },
                    {
                    "name": "baz",
                    "version": "9.8.7",
                    "type": "npm",
                    "last_updated": 1603311564,
                    "license": null,
                    "package_score": 0.75,
                    "status": "complete",
                    "vulnerabilities": [],
                    "heuristics": [
                        {
                        "data": null,
                        "score": 42
                        }
                    ]
                    }]}]}"#;

        let mock_server = build_mock_server().await;
        Mock::given(method("GET"))
            .and(path_regex(r"^/api/v0/job/[-\dabcdef]+".to_string()))
            .and(query_param("verbose", "True"))
            .respond_with_fn(move |_| ResponseTemplate::new(200).set_body_string(body))
            .mount(&mock_server)
            .await;

        let mut client = build_phylum_api(&mock_server).await?;

        let job = JobId::from_str("59482a54-423b-448d-8325-f171c9dc336b").unwrap();
        client.get_job_status_ext(&job).await?;

        Ok(())
    }

    #[tokio::test]
    async fn cancel() -> Result<(), PhylumApiError> {
        let body = r#"{"msg": "Job deleted"}"#;

        let mock_server = build_mock_server().await;
        Mock::given(method("DELETE"))
            .and(path_regex(
                r"^/api/v0/job/[-\dabcdef]+$".to_string().to_string(),
            ))
            .respond_with_fn(move |_| ResponseTemplate::new(200).set_body_string(body))
            .mount(&mock_server)
            .await;

        let mut client = build_phylum_api(&mock_server).await?;

        let job = JobId::from_str("59482a54-423b-448d-8325-f171c9dc336b").unwrap();
        client.cancel(&job).await?;

        Ok(())
    }
}
