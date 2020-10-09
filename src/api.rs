use crate::types::*;
use crate::restson::{Error, RestClient};


pub struct PhylumApi {
    client: RestClient,
}

impl PhylumApi {
    pub fn new(base_url: &str) -> PhylumApi {
        let client = RestClient::new(base_url).unwrap();
        PhylumApi { client }
    }

    // TODO: expose api functions in both blocking / async forms
    pub fn authenticate(&mut self, login: &str, password: &str) -> Result<Token, Error> {
        let req = AuthRequest{ 
            login: login.to_owned(),
            password: password.to_owned(),
        };
        let resp : AuthResponse = self.client.post_capture((), &req)?;
        self.client.set_header("Authorization", &format!("Bearer {}", resp.token.access_token))?;
        Ok(resp.token)
    }

    pub fn submit_request(&mut self, package_list: Vec<PackageDescriptor>) -> Result<JobId, Error> {
        let req = PackageRequest{
            packages: package_list,
        };
        log::debug!("==> Sending package submission: {:?}", req);
        let resp : PackageSubmissionResponse = self.client.put_capture((), &req)?;
        Ok(resp.job_id)
    }

    pub fn poll_status(&mut self, job_id: JobId) -> Result<RequestStatusResponse, Error> {
        let resp: RequestStatusResponse = self.client.get(job_id)?;
        Ok(resp)
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_authenticate() {
        authenticate("joe", "mypass");
    }
}
