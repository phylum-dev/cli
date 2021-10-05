use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::restson::{Error, RestPath};
use crate::types::API_PATH;

/// PUT /settings/current-user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Threshold {
    pub action: String,
    pub active: bool,
    pub threshold: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProject {
    pub thresholds: HashMap<String, Threshold>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Setting {
    DefaultLabel(HashMap<String, String>),
    Project(UserProject),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserSettings {
    pub version: u32,
    pub projects: HashMap<String, Setting>,
}

impl UserSettings {
    /// Sets the threshold for the given risk domain.
    pub fn set_threshold(
        &mut self,
        project_id: String,
        name: String,
        threshold: i32,
        action: String,
    ) {
        log::debug!("Retrieving user settings for project: {}", project_id);
        let mut thresholds = self
            .projects
            .get(project_id.as_str())
            .map(|s| s.to_owned())
            .unwrap_or_else(|| {
                Setting::Project(UserProject {
                    thresholds: HashMap::new(),
                })
            });

        if let Setting::Project(ref mut t) = thresholds {
            t.thresholds.insert(
                name,
                Threshold {
                    action,
                    active: (threshold > 0),
                    threshold: (threshold as f32) / 100.0,
                },
            );
        }

        self.projects.insert(project_id, thresholds);
    }
}

impl RestPath<()> for UserSettings {
    fn get_path(_: ()) -> Result<String, Error> {
        Ok(format!("{}/settings/current-user", API_PATH))
    }
}
