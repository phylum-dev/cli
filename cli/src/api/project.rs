use super::common::API_PATH;

pub(crate) fn get_project_summary(api_uri: &str) -> String {
    format!("{api_uri}/{API_PATH}/job/projects/overview")
}

// impl RestPath<()> for Vec<ProjectSummaryResponse> {
//     fn get_path(_: ()) -> Result<String, Error> {
//         Ok(format!("{}/job/projects/overview", API_PATH))
//     }
// }

pub(crate) fn put_create_project(api_uri: &str) -> String {
    format!("{api_uri}/{API_PATH}/job/projects")
}

// impl RestPath<()> for CreateProjectRequest {
//     fn get_path(_: ()) -> Result<String, Error> {
//         Ok(format!("{}/job/projects", API_PATH))
//     }
// }
