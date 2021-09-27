use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use super::common::*;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageType {
    Npm,
    Python,
    Java,
    Ruby,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum RiskLevel {
    #[serde(rename = "critical")]
    Crit,
    #[serde(rename = "high")]
    High,
    #[serde(rename = "medium")]
    Med,
    #[serde(rename = "low")]
    Low,
    #[serde(rename = "info")]
    Info,
}

impl fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let risk_level = format!("{:?}", self);
        write!(f, "{}", risk_level.to_lowercase())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum RiskDomain {
    MaliciousCode,
    Vulnerabilities,
    EngineeringRisk,
    AuthorRisk,
    LicenseRisk,
}

impl fmt::Display for RiskDomain {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let risk_domain = match self {
            RiskDomain::MaliciousCode => "MAL",
            RiskDomain::Vulnerabilities => "VLN",
            RiskDomain::EngineeringRisk => "ENG",
            RiskDomain::AuthorRisk => "AUT",
            RiskDomain::LicenseRisk => "LIC",
        };
        write!(f, "{}", risk_domain)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Issue {
    pub title: String,
    pub description: String,
    pub risk_level: RiskLevel,
    pub risk_domain: RiskDomain,
    pub pkg_name: String,
    pub pkg_version: String,
    pub score: f64,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct HeuristicResult {
    pub domain: RiskDomain,
    pub score: f64,
    pub risk_level: RiskLevel,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct Vulnerability {
    pub cve: Vec<String>,
    pub base_severity: f32,
    pub risk_level: RiskLevel,
    pub title: String,
    pub description: String,
    pub remediation: String,
}

impl FromStr for PackageType {
    type Err = ();

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input.to_lowercase().as_str() {
            "npm" => Ok(Self::Npm),
            "python" => Ok(Self::Python),
            "java" => Ok(Self::Java),
            "ruby" => Ok(Self::Ruby),
            _ => Err(()),
        }
    }
}

impl fmt::Display for PackageType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let package_type = format!("{:?}", self);
        write!(f, "{}", package_type.to_lowercase())
    }
}

impl PackageType {
    pub fn language(&self) -> &str {
        match self {
            PackageType::Npm => "Javascript",
            PackageType::Ruby => "Ruby",
            PackageType::Python => "Python",
            PackageType::Java => "Java",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackageDescriptor {
    pub name: String,
    pub version: String,
    pub r#type: PackageType,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PackageStatus {
    pub name: String,
    pub version: String,
    pub status: Status,
    pub last_updated: u64,
    pub license: Option<String>,
    pub package_score: Option<f64>,
    pub num_dependencies: u32,
    pub num_vulnerabilities: u32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PackageStatusExtended {
    #[serde(flatten)]
    pub basic_status: PackageStatus,
    pub r#type: PackageType,
    #[serde(rename = "riskVectors")]
    pub risk_vectors: HashMap<String, f64>,
    pub dependencies: Vec<PackageDescriptor>,
    pub vulnerabilities: Vec<Vulnerability>,
    pub heuristics: HashMap<String, HeuristicResult>,
    pub issues: Vec<Issue>,
}
