use crate::restson::{Error, RestClient};
use crate::types::*;

pub struct PhylumApi {
    client: RestClient,
}

impl PhylumApi {
    pub fn new(base_url: &str) -> Result<PhylumApi, Error> {
        let client = RestClient::new(base_url)?;
        Ok(PhylumApi { client })
    }

    // TODO: expose api functions in both blocking / async forms

    /// Authenticate to the system
    pub fn authenticate(&mut self, login: &str, password: &str) -> Result<Token, Error> {
        let req = AuthRequest {
            login: login.to_owned(),
            password: password.to_owned(),
        };
        let resp: AuthResponse = self.client.post_capture((), &req)?;
        self.client.set_header(
            "Authorization",
            &format!("Bearer {}", resp.token.access_token),
        )?;
        Ok(resp.token)
    }

    /// Submit a package request to the system
    pub fn submit_request(&mut self, package_list: &[PackageDescriptor]) -> Result<JobId, Error> {
        let req = PackageRequest {
            packages: package_list.to_vec(),
        };
        log::debug!("==> Sending package submission: {:?}", req);
        let resp: PackageSubmissionResponse = self.client.put_capture((), &req)?;
        Ok(resp.job_id)
    }

    /// Get the status of a previously submitted job
    pub fn get_status(&mut self, job_id: &JobId) -> Result<RequestStatusResponse, Error> {
        let resp: RequestStatusResponse = self.client.get(job_id.to_owned())?;
        Ok(resp)
    }

    /// Cancel a job currently in progress
    pub fn cancel(&mut self, job_id: &JobId) -> Result<CancelRequestResponse, Error> {
        let resp: CancelRequestResponse = self.client.delete_capture(job_id.to_owned())?;
        Ok(resp)
    }
}

#[cfg(test)]
mod tests {
    use mockito::{mock, Matcher};
    use std::str::FromStr;

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
        let _m = mock("POST", "/auth/login")
            .with_status(200)
            .with_header("content-type", "application-json")
            .with_body(r#"{"access_token": "abcd1234", "refresh_token": "23456789"}"#)
            .create();

        let mut client = PhylumApi::new(&mockito::server_url()).unwrap();
        let res = client.authenticate("joe", "mypass");
        assert!(res.is_ok(), format!("{:?}", res));
    }

    #[test]
    fn submit_request() {
        let _m = mock("PUT", "/request/package")
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
        let res = client.submit_request(&[pkg]);
        assert!(res.is_ok(), format!("{:?}", res));
    }

    #[test]
    fn get_status() {
        let _m = mock(
            "GET",
            Matcher::Regex(r"^/request/package/[-\dabcdef]+$".to_string()),
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
                    "risk": 60.0,
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
                        "risk": 60.0,
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
                        "risk": 60.0,
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
        let res = client.get_status(&job);
        assert!(res.is_ok(), format!("{:?}", res));
    }

    #[test]
    fn cancel() {
        let _m = mock(
            "DELETE",
            Matcher::Regex(r"^/request/package/[-\dabcdef]+$".to_string()),
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
}
