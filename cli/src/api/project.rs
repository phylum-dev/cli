use super::common::API_PATH;

pub(crate) fn get_project_summary(api_uri: &str) -> String {
    format!("{api_uri}/{API_PATH}/job/projects/overview")
}

pub(crate) fn put_create_project(api_uri: &str) -> String {
    format!("{api_uri}/{API_PATH}/job/projects")
}
