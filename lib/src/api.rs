use crate::restson::{Error, RestClient};
use crate::types::*;
use std::time::Duration;

pub struct PhylumApi {
    client: RestClient,
    pub api_key: Option<ApiToken>,
}

impl PhylumApi {
    pub fn new(base_url: &str, request_timeout: Option<u64>) -> Result<PhylumApi, Error> {
        let timeout = request_timeout.unwrap_or(30);
        log::debug!("Setting request timeout to {} seconds", timeout);

        let mut client = RestClient::builder()
            .timeout(Duration::from_secs(timeout))
            .build(base_url)?;

        let yml = clap::load_yaml!("bin/.conf/cli.yaml");
        let version = yml["version"].as_str().unwrap_or("");
        client.set_header("version", version)?;

        Ok(PhylumApi {
            client,
            api_key: None,
        })
    }

    // TODO: expose api functions in both blocking / async forms

    /// Ping the system and verify it's up
    pub fn ping(&mut self) -> Result<String, Error> {
        let resp: PingResponse = self.client.get(())?;
        Ok(resp.msg)
    }

    /// Create a new project
    pub fn create_project(&mut self, name: &str) -> Result<ProjectId, Error> {
        let req = ProjectCreateRequest {
            name: name.to_string(),
        };
        let resp: ProjectCreateResponse = self.client.put_capture((), &req)?;
        Ok(resp.id)
    }

    /// Get a list of projects
    pub fn get_projects(&mut self) -> Result<Vec<ProjectGetRequest>, Error> {
        let resp: Vec<ProjectGetRequest> = self.client.get(())?;
        Ok(resp)
    }

    /// Get user settings
    pub fn get_user_settings(&mut self) -> Result<UserSettings, Error> {
        let resp: UserSettings = self.client.get(())?;
        Ok(resp)
    }

    /// Put updated user settings
    pub fn put_user_settings(&mut self, settings: &UserSettings) -> Result<bool, Error> {
        let _resp: UserSettings = self.client.put_capture((), settings)?;
        Ok(true)
    }

    /// Register new user
    pub fn register(&mut self, email: &str, password: &str, name: &str) -> Result<UserId, Error> {
        let req = RegisterRequest {
            email: email.to_owned(),
            password: password.to_owned(),
            confirm_password: password.to_owned(),
            first_name: name.to_owned(),
            last_name: name.to_owned(),
        };

        let resp: RegisterResponse = self.client.put_capture((), &req)?;
        Ok(resp.user_id)
    }

    /// Authenticate to the system and receive a JWT token
    pub fn authenticate(&mut self, email: &str, password: &str) -> Result<JwtToken, Error> {
        let req = AuthRequest {
            email: email.to_owned(),
            password: password.to_owned(),
        };
        let resp: AuthResponse = self.client.post_capture((), &req)?;
        self.client.set_jwt_auth(&resp.token.access_token)?;
        Ok(resp.token)
    }

    /// Refresh the current JWT token
    pub fn refresh(&mut self, token: &JwtToken) -> Result<JwtToken, Error> {
        let refresh_token = token
            .refresh_token
            .as_ref()
            .ok_or("Missing refresh token")?;
        self.client.set_jwt_auth(&refresh_token)?;
        let req = RefreshRequest {};
        let resp: AuthResponse = self.client.post_capture((), &req)?;
        self.client.set_jwt_auth(&resp.token.access_token)?;
        Ok(resp.token)
    }

    /// Create a long-lived API token for later use
    pub fn create_api_token(&mut self) -> Result<ApiToken, Error> {
        let req = ApiCreateTokenRequest {};
        let resp: ApiToken = self.client.put_capture((), &req)?;
        Ok(resp)
    }

    /// Delete (deactivate) an API token
    pub fn delete_api_token(&mut self, key: &Key) -> Result<(), Error> {
        let req = ApiDeleteTokenRequest {};
        self.client.delete(key.to_owned(), &req)?;
        Ok(())
    }

    /// Retrieve all API tokens
    pub fn get_api_tokens(&mut self) -> Result<Vec<ApiToken>, Error> {
        let resp: GetApiTokensResponse = self.client.get(())?;
        Ok(resp.keys)
    }

    /// Set the API token to use for requests to the `/job` endpoint
    pub fn set_api_token(&mut self, token: &ApiToken) -> Result<(), Error> {
        self.api_key = Some(token.to_owned());

        // Remove any existing JWT auth header
        self.client.clear_headers();
        // Set the `apikey` header to use for authentication
        self.client.set_api_key(&token.key.to_string())
    }

    /// Submit a new request to the system
    pub fn submit_request(
        &mut self,
        req_type: &PackageType,
        package_list: &[PackageDescriptor],
        is_user: bool,
        project: ProjectId,
        label: Option<String>,
    ) -> Result<JobId, Error> {
        let req = PackageRequest {
            r#type: req_type.to_owned(),
            packages: package_list.to_vec(),
            is_user,
            project,
            label: label.unwrap_or_else(|| "uncategorized".to_string()),
        };
        log::debug!("==> Sending package submission: {:?}", req);
        let resp: PackageSubmissionResponse = self.client.put_capture((), &req)?;
        Ok(resp.job_id)
    }

    /// Get the status of a previously submitted job
    pub fn get_job_status(
        &mut self,
        job_id: &JobId,
    ) -> Result<RequestStatusResponse<PackageStatus>, Error> {
        let resp: RequestStatusResponse<PackageStatus> = self.client.get(job_id.to_owned())?;
        Ok(resp)
    }

    /// Get the status of a previously submitted job (verbose output)
    pub fn get_job_status_ext(
        &mut self,
        job_id: &JobId,
    ) -> Result<RequestStatusResponse<PackageStatusExtended>, Error> {
        let resp: RequestStatusResponse<PackageStatusExtended> =
            self.client.get(job_id.to_owned())?;
        Ok(resp)
    }

    /// Get the status of all jobs
    pub fn get_status(&mut self) -> Result<AllJobsStatusResponse, Error> {
        let resp: AllJobsStatusResponse = self.client.get(30)?;
        Ok(resp)
    }

    /// Get the details of a specific project
    pub fn get_project_details(
        &mut self,
        project_name: &str,
    ) -> Result<ProjectGetDetailsRequest, Error> {
        let resp: ProjectGetDetailsRequest = self.client.get(project_name)?;
        Ok(resp)
    }

    /// Get package details
    pub fn get_package_details(
        &mut self,
        pkg: &PackageDescriptor,
    ) -> Result<PackageStatusExtended, Error> {
        let resp: PackageStatusExtended = self.client.get(pkg.to_owned())?;
        Ok(resp)
    }

    /// Cancel a job currently in progress
    pub fn cancel(&mut self, job_id: &JobId) -> Result<CancelRequestResponse, Error> {
        let resp: CancelRequestResponse = self.client.delete_capture(job_id.to_owned())?;
        Ok(resp)
    }
}

/// Tests
#[cfg(test)]
mod tests {
    use mockito::{mock, Matcher};
    use std::str::FromStr;
    use uuid::Uuid;

    use super::*;
    #[test]
    fn create_client() {
        let client = PhylumApi::new("http://127.0.0.1", None);
        assert!(client.is_ok());
    }

    #[test]
    fn create_client_should_fail() {
        let client = PhylumApi::new("not_a_real_url.123", None);
        assert!(client.is_err());
    }

    #[test]
    fn authenticate() {
        let _m = mock("POST", "/api/v0/authenticate/login")
            .with_status(200)
            .with_header("content-type", "application-json")
            .with_body(r#"{"access_token": "abcd1234", "refresh_token": "23456789"}"#)
            .create();

        let mut client = PhylumApi::new(&mockito::server_url(), None).unwrap();
        let res = client.authenticate("joe", "mypass");
        assert!(res.is_ok(), "{:?}", res);
    }

    #[test]
    fn refresh() {
        let _m = mock("POST", "/api/v0/authenticate/refresh")
            .with_status(200)
            .with_header("content-type", "application-json")
            .with_body(r#"{"access_token": "abcd1234", "refresh_token": "23456789"}"#)
            .create();

        let mut client = PhylumApi::new(&mockito::server_url(), None).unwrap();
        let jwt = JwtToken {
            access_token: "abcd1234".to_string(),
            refresh_token: Some("abcd1234".to_string()),
        };
        let res = client.refresh(&jwt);
        assert!(res.is_ok(), "{:?}", res);
    }

    #[test]
    fn submit_request() {
        let _m = mock("PUT", "/api/v0/job")
            .with_status(201)
            .with_header("content-type", "application-json")
            .with_body(r#"{"job_id": "59482a54-423b-448d-8325-f171c9dc336b"}"#)
            .create();

        let mut client = PhylumApi::new(&mockito::server_url(), None).unwrap();
        let pkg = PackageDescriptor {
            name: "react".to_string(),
            version: "16.13.1".to_string(),
            r#type: PackageType::Npm,
        };
        let project_id = Uuid::new_v4();
        let label = Some("mylabel".to_string());
        let res = client.submit_request(&PackageType::Npm, &[pkg], true, project_id, label);
        assert!(res.is_ok(), "{:?}", res);
    }

    #[test]
    fn get_status() {
        let _m = mock("GET", "/api/v0/job/?limit=30&verbose=1")
            .with_status(200)
            .with_header("content-type", "application-json")
            .with_body(
                r#"
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
                            "project": "test-project",
                            "total_jobs": 1,
                            "score": 1.0,
                            "ecosystem": "npm"
                        }

                    ],
                    "total_jobs": 1
                }"#,
            )
            .create();

        let mut client = PhylumApi::new(&mockito::server_url(), None).unwrap();
        let res = client.get_status();
        assert!(res.is_ok(), "{:?}", res);
    }

    #[test]
    fn get_package_details() {
        let _m = mock("GET", "/api/v0/job/packages/npm/@schematics~angular/9.1.9")
            .with_status(200)
            .with_header("content-type", "application-json")
            .with_body(
                r#"
            {
                "name": "@schematics/angular",
                "version": "9.1.9",
                "type": "npm",
                "last_updated": 1611962723183,
                "license": "MIT",
                "package_score": 1.0,
                "num_dependencies": 2,
                "num_vulnerabilities": 4,
                "status": "complete",
                "vulnerabilities": [],
                "riskVectors": {
                    "author": 0.90,
                    "engineering": 0.42,
                    "license": 1.0,
                    "malicious_code": 1.0,
                    "vulnerability": 1.0
                },
                "heuristics": {
                    "something": {
                        "description": "do stuff",
                        "score": 3.14,
                        "domain": "AuthorRisk"
                    }
                },
                "dependencies": []
              }
            "#,
            )
            .create();

        let mut client = PhylumApi::new(&mockito::server_url(), None).unwrap();
        let pkg = PackageDescriptor {
            name: "@schematics/angular".to_string(),
            version: "9.1.9".to_string(),
            r#type: PackageType::Npm,
        };
        let res = client.get_package_details(&pkg);
        assert!(res.is_ok(), "{:?}", res);
    }

    #[test]
    fn get_job_status() {
        let _m = mock(
            "GET",
            Matcher::Regex(r"^/api/v0/job/[-\dabcdef]+$".to_string()),
        )
        .with_status(200)
        .with_header("content-type", "application-json")
        .with_body(
            r#"
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
                    }]}"#,
        )
        .create();

        let mut client = PhylumApi::new(&mockito::server_url(), None).unwrap();
        let job = JobId::from_str("59482a54-423b-448d-8325-f171c9dc336b").unwrap();
        let res = client.get_job_status(&job);
        assert!(res.is_ok(), "{:?}", res);
    }

    #[test]
    fn get_job_status_ext() {
        let _m = mock(
            "GET",
            Matcher::Regex(r"^/api/v0/job/[-\dabcdef]+\?verbose=True$".to_string()),
        )
        .with_status(200)
        .with_header("content-type", "application-json")
        .with_body(
            r#"
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
                    "heuristics": {
                        "something": {
                            "description": "do stuff",
                            "score": 3.14,
                            "domain": "EngineeringRisk"
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
                        }]}]}"#,
        )
        .create();

        let mut client = PhylumApi::new(&mockito::server_url(), None).unwrap();
        let job = JobId::from_str("59482a54-423b-448d-8325-f171c9dc336b").unwrap();
        let res = client.get_job_status_ext(&job);
        assert!(res.is_ok(), "{:?}", res);
    }

    #[test]
    fn cancel() {
        let _m = mock(
            "DELETE",
            Matcher::Regex(r"^/api/v0/job/[-\dabcdef]+$".to_string()),
        )
        .with_status(200)
        .with_header("content-type", "application-json")
        .with_body(r#"{"msg": "Job deleted"}"#)
        .create();

        let mut client = PhylumApi::new(&mockito::server_url(), None).unwrap();
        let job = JobId::from_str("59482a54-423b-448d-8325-f171c9dc336b").unwrap();
        let res = client.cancel(&job);
        assert!(res.is_ok(), "{:?}", res);
    }

    #[test]
    fn register() {
        let _m = mock("PUT", "/api/v0/authenticate/register")
            .with_status(201)
            .with_header("content-type", "application-json")
            .with_body(
                r#"
            { "email": "johnsmith@somedomain.com",
              "first_name": "John",
              "last_name":  "Smith",
              "role":  "a",
              "user_id": "abcd1234-abcd-1234-5678-abcd12345678"
            }
        "#,
            )
            .create();

        let mut client = PhylumApi::new(&mockito::server_url(), None).unwrap();
        let res = client.register("johnsmith@somedomain.com", "agreatpassword", "john smith");
        assert!(res.is_ok(), "{:?}", res);
    }

    #[test]
    fn create_token() {
        let _m = mock("PUT", "/api/v0/authenticate/key")
            .with_status(201)
            .with_header("content-type", "application-json")
            .with_body(
                r#"{
                "active": true,
                "key": "a37ba84d-67b4-42ff-910e-25ec5fb7b909",
                "user_id": "f8becb8d-f0e7-4420-9249-053d8228b19e",
                "created": "Dec 28, 2017",
                "name": "foobar"
            }"#,
            )
            .create();

        let mut client = PhylumApi::new(&mockito::server_url(), None).unwrap();
        let res = client.create_api_token();
        assert!(res.is_ok(), "{:?}", res);
        let token = res.unwrap();
        assert_eq!(token.active, true);
        assert_eq!(
            token.key,
            Key::from_str("a37ba84d-67b4-42ff-910e-25ec5fb7b909").unwrap()
        );
        assert_eq!(
            token.user_id,
            UserId::from_str("f8becb8d-f0e7-4420-9249-053d8228b19e").unwrap()
        );
    }

    #[test]
    fn delete_token() {
        let _m = mock(
            "DELETE",
            Matcher::Regex(r"^/api/v0/authenticate/key/[-\dabcdef]+$".to_string()),
        )
        .with_status(200)
        .with_header("content-type", "application-json")
        .create();

        let mut client = PhylumApi::new(&mockito::server_url(), None).unwrap();
        let key = Key::from_str("b75e1f40-02a5-4580-a7d1-d842dbcc1aca").unwrap();
        let res = client.delete_api_token(&key);
        assert!(res.is_ok(), "{:?}", res);
    }

    #[test]
    fn get_tokens() {
        let _m = mock("GET", "/api/v0/authenticate/key")
            .with_status(200)
            .with_header("content-type", "application-json")
            .with_body(
                r#"
            {
                "keys": [
                {
                    "active": true,
                    "key": "a37ba84d-67b4-42ff-910e-25ec5fb7b909",
                    "user_id": "f8becb8d-f0e7-4420-9249-053d8228b19e",
                    "created": "Jan 12, 1988",
                    "name": "Spameggs"
                },
                {
                    "active": false,
                    "key": "b37ba84d-67b4-42ff-910e-25ec5fb7b909",
                    "user_id": "e8becb8d-f0e7-4420-9249-053d8228b19e",
                    "created": "Nov 16, 1959",
                    "name": "test"
                }
                ]
            }"#,
            )
            .create();

        let mut client = PhylumApi::new(&mockito::server_url(), None).unwrap();
        let res = client.get_api_tokens();

        assert!(res.is_ok(), "{:?}", res);
    }
}
