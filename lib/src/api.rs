use crate::restson::{Error, RestClient};
use crate::types::*;

pub struct PhylumApi {
    client: RestClient,
    pub api_key: Option<ApiToken>,
}

impl PhylumApi {
    pub fn new(base_url: &str) -> Result<PhylumApi, Error> {
        let mut client = RestClient::new(base_url)?;
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

    /// Register new user
    pub fn register(
        &mut self,
        email: &str,
        password: &str,
        first_name: &str,
        last_name: &str,
    ) -> Result<UserId, Error> {
        let req = RegisterRequest {
            email: email.to_owned(),
            password: password.to_owned(),
            confirm_password: password.to_owned(),
            first_name: first_name.to_owned(),
            last_name: last_name.to_owned(),
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
        no_recurse: bool,
        project: Option<ProjectId>,
        label: Option<String>,
    ) -> Result<JobId, Error> {
        let req = PackageRequest {
            r#type: req_type.to_owned(),
            packages: package_list.to_vec(),
            is_user,
            norecurse: no_recurse,
            project,
            label,
        };
        log::debug!("==> Sending package submission: {:?}", req);
        let resp: PackageSubmissionResponse = self.client.put_capture((), &req)?;
        Ok(resp.job_id)
    }

    /// Get the status of a previously submitted job
    pub fn get_job_status(&mut self, job_id: &JobId) -> Result<RequestStatusResponse, Error> {
        let resp: RequestStatusResponse = self.client.get(job_id.to_owned())?;
        Ok(resp)
    }

    /// Get the status of all jobs
    pub fn get_status(&mut self) -> Result<Vec<JobDescriptor>, Error> {
        let resp: AllJobsStatusResponse = self.client.get(())?;
        Ok(resp.jobs)
    }

    /// Get package details
    pub fn get_package_details(&mut self, pkg: &PackageDescriptor) -> Result<PackageStatus, Error> {
        let resp: PackageStatus = self.client.get(pkg.to_owned())?;
        Ok(resp)
    }

    /// Cancel a job currently in progress
    pub fn cancel(&mut self, job_id: &JobId) -> Result<CancelRequestResponse, Error> {
        let resp: CancelRequestResponse = self.client.delete_capture(job_id.to_owned())?;
        Ok(resp)
    }

    /// List available heuristics
    pub fn query_heuristics(&mut self) -> Result<Vec<String>, Error> {
        let resp: HeuristicsListResponse = self.client.get(())?;
        Ok(resp.heuristics)
    }

    /// Submit a package / packages to have heuristics run against
    pub fn submit_heuristics(
        &mut self,
        pkg: &PackageDescriptor,
        heuristics: &[String],
        include_deps: bool,
    ) -> Result<String, Error> {
        let req = HeuristicsSubmitRequest {
            package: pkg.to_owned(),
            heuristics_filter: heuristics.to_vec(),
            include_deps,
        };
        log::debug!("==> Submitting heuristics run: {:?}", req);
        let resp: HeuristicsSubmitResponse = self.client.post_capture((), &req)?;
        Ok(resp.msg)
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
        let client = PhylumApi::new("http://127.0.0.1");
        assert!(client.is_ok());
    }

    #[test]
    fn create_client_should_fail() {
        let client = PhylumApi::new("not_a_real_url.123");
        assert!(client.is_err());
    }

    #[test]
    fn authenticate() {
        let _m = mock("POST", "/api/v0/authenticate/login")
            .with_status(200)
            .with_header("content-type", "application-json")
            .with_body(r#"{"access_token": "abcd1234", "refresh_token": "23456789"}"#)
            .create();

        let mut client = PhylumApi::new(&mockito::server_url()).unwrap();
        let res = client.authenticate("joe", "mypass");
        assert!(res.is_ok(), format!("{:?}", res));
    }

    #[test]
    fn refresh() {
        let _m = mock("POST", "/api/v0/authenticate/refresh")
            .with_status(200)
            .with_header("content-type", "application-json")
            .with_body(r#"{"access_token": "abcd1234", "refresh_token": "23456789"}"#)
            .create();

        let mut client = PhylumApi::new(&mockito::server_url()).unwrap();
        let jwt = JwtToken {
            access_token: "abcd1234".to_string(),
            refresh_token: Some("abcd1234".to_string()),
        };
        let res = client.refresh(&jwt);
        assert!(res.is_ok(), format!("{:?}", res));
    }

    #[test]
    fn submit_request() {
        let _m = mock("PUT", "/api/v0/job")
            .with_status(201)
            .with_header("content-type", "application-json")
            .with_body(r#"{"job_id": "59482a54-423b-448d-8325-f171c9dc336b"}"#)
            .create();

        let mut client = PhylumApi::new(&mockito::server_url()).unwrap();
        let pkg = PackageDescriptor {
            name: "react".to_string(),
            version: "16.13.1".to_string(),
            r#type: PackageType::Npm,
        };
        let project_id = Uuid::new_v4();
        let label = Some("mylabel".to_string());
        let res = client.submit_request(
            &PackageType::Npm,
            &[pkg],
            true,
            true,
            Some(project_id),
            label,
        );
        assert!(res.is_ok(), format!("{:?}", res));
    }

    #[test]
    fn get_status() {
        let _m = mock("GET", "/api/v0/job")
            .with_status(200)
            .with_header("content-type", "application-json")
            .with_body(
                r#"
            [[
                {
                  "job_id": "f8e8cb21-a4c0-4718-9cd2-8f631e95b951",
                  "packages": [
                    {
                      "name": "esmalo",
                      "version": "1.0.0",
                      "type": "npm"
                    }
                  ]
                },
                {
                  "job_id": "1d1ecd1d-94af-4841-bf55-f5506b4f8f9f",
                  "packages": [
                    {
                      "name": "esmalo",
                      "version": "1.0.0",
                      "type": "npm"
                    }
                  ]
                }
            ]]"#,
            )
            .create();

        let mut client = PhylumApi::new(&mockito::server_url()).unwrap();
        let res = client.get_status();
        assert!(res.is_ok(), format!("{:?}", res));
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
                "status": "PENDING_EXTERNAL_PROCESSING",
                "vulnerabilities": [],
                "heuristics": {
                  "sample": {
                    "data": {},
                    "raw_score": 10.192523726982682,
                    "score": 0.9999625521468548
                  }
                },
                "dependencies": []
              }
            "#,
            )
            .create();

        let mut client = PhylumApi::new(&mockito::server_url()).unwrap();
        let pkg = PackageDescriptor {
            name: "@schematics/angular".to_string(),
            version: "9.1.9".to_string(),
            r#type: PackageType::Npm,
        };
        let res = client.get_package_details(&pkg);
        assert!(res.is_ok(), format!("{:?}", res));
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
                "id": "59482a54-423b-448d-8325-f171c9dc336b",
                "user_id": "86bb664a-5331-489b-8901-f052f155ec79",
                "started_at": 1603311564,
                "last_updated": 1603311584,
                "status": "NEW",
                "packages": [
                    {
                    "name": "foo",
                    "version": "1.0.0",
                    "type": "npm",
                    "last_updated": 1603311564,
                    "license": null,
                    "package_score": 60.0,
                    "status": "NEW",
                    "vulnerabilities": [],
                    "heuristics": [
                        {
                        "data": {
                            "foo": "bar"
                        },
                        "score": 3.14
                        }
                    ],
                    "dependencies": [
                        {
                        "name": "bar",
                        "version": "2.3.4",
                        "type": "npm",
                        "last_updated": 1603311564,
                        "license": null,
                        "package_score": 60.0,
                        "status": "COMPLETED",
                        "vulnerabilities": [],
                        "heuristics": []
                        },
                        {
                        "name": "baz",
                        "version": "9.8.7",
                        "type": "npm",
                        "last_updated": 1603311564,
                        "license": null,
                        "package_score": 60.0,
                        "status": "NEW",
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

        let mut client = PhylumApi::new(&mockito::server_url()).unwrap();
        let job = JobId::from_str("59482a54-423b-448d-8325-f171c9dc336b").unwrap();
        let res = client.get_job_status(&job);
        assert!(res.is_ok(), format!("{:?}", res));
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

        let mut client = PhylumApi::new(&mockito::server_url()).unwrap();
        let job = JobId::from_str("59482a54-423b-448d-8325-f171c9dc336b").unwrap();
        let res = client.cancel(&job);
        assert!(res.is_ok(), format!("{:?}", res));
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

        let mut client = PhylumApi::new(&mockito::server_url()).unwrap();
        let res = client.register(
            "johnsmith@somedomain.com",
            "agreatpassword",
            "john",
            "smith",
        );
        assert!(res.is_ok(), format!("{:?}", res));
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
                "user_id": "f8becb8d-f0e7-4420-9249-053d8228b19e"
            }"#,
            )
            .create();

        let mut client = PhylumApi::new(&mockito::server_url()).unwrap();
        let res = client.create_api_token();
        assert!(res.is_ok(), format!("{:?}", res));
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

        let mut client = PhylumApi::new(&mockito::server_url()).unwrap();
        let key = Key::from_str("b75e1f40-02a5-4580-a7d1-d842dbcc1aca").unwrap();
        let res = client.delete_api_token(&key);
        assert!(res.is_ok(), format!("{:?}", res));
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
                    "user_id": "f8becb8d-f0e7-4420-9249-053d8228b19e"
                },
                {
                    "active": false,
                    "key": "b37ba84d-67b4-42ff-910e-25ec5fb7b909",
                    "user_id": "e8becb8d-f0e7-4420-9249-053d8228b19e"
                }
                ]
            }"#,
            )
            .create();

        let mut client = PhylumApi::new(&mockito::server_url()).unwrap();
        let res = client.get_api_tokens();

        assert!(res.is_ok(), format!("{:?}", res));
    }

    #[test]
    fn list_heuristics() {
        let _m = mock("GET", "/api/v0/job/heuristics")
            .with_status(200)
            .with_header("content-type", "application-json")
            .with_body(r#"{"heuristics": ["some_heuristic", "esmalo", "typosquatting"]}"#)
            .create();

        let mut client = PhylumApi::new(&mockito::server_url()).unwrap();
        let res = client.query_heuristics();
        assert!(res.is_ok(), format!("{:?}", res));
    }

    #[test]
    fn submit_heuristics() {
        let _m = mock("POST", "/api/v0/job/heuristics")
            .with_status(200)
            .with_header("content-type", "application-json")
            .with_body(r#"{"msg": "ok"}"#)
            .create();

        let mut client = PhylumApi::new(&mockito::server_url()).unwrap();
        let pkg = PackageDescriptor {
            name: "react".to_string(),
            version: "16.13.1".to_string(),
            r#type: PackageType::Npm,
        };
        let heuristics = vec!["some_heuristic", "esmalo", "typosquatting"]
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();

        let res = client.submit_heuristics(&pkg, &heuristics, true);
        assert!(res.is_ok(), format!("{:?}", res));
    }
}
