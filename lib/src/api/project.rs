use phylum_types::types::project::*;

use super::common::API_PATH;
use crate::restson::{Error, RestPath};

impl RestPath<()> for Vec<ProjectSummaryResponse> {
    fn get_path(_: ()) -> Result<String, Error> {
        Ok(format!("{}/job/projects/overview", API_PATH))
    }
}

impl RestPath<()> for CreateProjectRequest {
    fn get_path(_: ()) -> Result<String, Error> {
        Ok(format!("{}/job/projects", API_PATH))
    }
}
