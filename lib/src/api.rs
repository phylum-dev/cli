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
    use super::*;
    #[test]
    fn create_client() {
        let client = PhylumApi::new("http://127.0.0.1");
        assert!(client.is_ok());
    }
    #[test]
    fn authenticate() {
        //authenticate("joe", "mypass");
    }
}
