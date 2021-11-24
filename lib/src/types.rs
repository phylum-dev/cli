use std::str::FromStr;

use serde::{Deserialize, Serialize};

mod auth;
mod common;
mod job;
mod package;
mod project;
mod user_settings;

pub use auth::*;
pub use common::*;
pub use job::*;
pub use package::*;
pub use project::*;
pub use user_settings::*;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Admin,
    User,
    Observer,
}

impl FromStr for Role {
    type Err = ();

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "administrator" => Ok(Self::Admin),
            "observer" => Ok(Self::Observer),
            "user" => Ok(Self::User),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct GithubRelease {
    pub name: String,
    pub assets: Vec<GithubReleaseAsset>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GithubReleaseAsset {
    pub browser_download_url: String,
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_level_ordering() {
        assert!(
            RiskLevel::Info < RiskLevel::Low
                && RiskLevel::Low < RiskLevel::Med
                && RiskLevel::Med < RiskLevel::High
                && RiskLevel::High < RiskLevel::Crit,
            "Ordering of risk levels is invalid"
        );
    }
}
