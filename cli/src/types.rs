use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct PingResponse {
    pub response: String,
}

// TODO Deprecate
#[derive(Debug, Serialize, Deserialize)]
pub struct AuthStatusResponse {
    pub authenticated: bool,
}

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
    use phylum_types::types::package::RiskLevel;

    #[test]
    fn test_risk_level_ordering() {
        assert!(
            RiskLevel::Info < RiskLevel::Low
                && RiskLevel::Low < RiskLevel::Medium
                && RiskLevel::Medium < RiskLevel::High
                && RiskLevel::High < RiskLevel::Critical,
            "Ordering of risk levels is invalid"
        );
    }
}
