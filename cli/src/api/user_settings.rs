use super::common::API_PATH;

pub(crate) fn get_user_settings(api_uri: &str) -> String {
    format!("{api_uri}/{API_PATH}/settings/current-user")
}
/// PUT /settings/current-user
pub(crate) fn put_user_settings(api_uri: &str) -> String {
    format!("{api_uri}/{API_PATH}/settings/current-user")
}
// impl RestPath<()> for UserSettings {
//     fn get_path(_: ()) -> Result<String, Error> {
//         Ok(format!("{}/settings/current-user", API_PATH))
//     }
// }
