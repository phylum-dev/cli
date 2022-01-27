use phylum_types::types::user_settings::UserSettings;

use super::common::API_PATH;
use crate::restson::{Error, RestPath};

/// PUT /settings/current-user
impl RestPath<()> for UserSettings {
    fn get_path(_: ()) -> Result<String, Error> {
        Ok(format!("{}/settings/current-user", API_PATH))
    }
}
