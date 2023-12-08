use std::collections::HashSet;
use std::iter::FromIterator;
use std::str::FromStr;

use phylum_types::types::job::JobStatusResponse;
use phylum_types::types::package::PackageStatusExtended;

use crate::types::{Package, RiskDomain, RiskLevel, RiskType};

/// Remove issues based on a filter.
pub trait FilterIssues {
    fn filter(&mut self, filter: &Filter);
}

impl FilterIssues for Package {
    fn filter(&mut self, filter: &Filter) {
        self.issues.retain(|issue| !should_filter_issue(filter, issue.impact, issue.risk_type));
    }
}

impl<T: FilterIssues> FilterIssues for JobStatusResponse<T> {
    fn filter(&mut self, filter: &Filter) {
        for package in &mut self.packages {
            package.filter(filter);
        }
    }
}

impl FilterIssues for PackageStatusExtended {
    fn filter(&mut self, filter: &Filter) {
        self.issues.retain(|issue| {
            !should_filter_issue(
                filter,
                issue.issue.severity.into(),
                RiskDomain::from(issue.issue.domain),
            )
        });
    }
}

/// Check if a package should be filtered out.
fn should_filter_issue(filter: &Filter, level: RiskLevel, risk_type: impl Into<RiskType>) -> bool {
    if let Some(filter_level) = filter.level {
        if level < filter_level {
            return true;
        }
    }

    if let Some(domains) = &filter.domains {
        if !domains.contains(&risk_type.into()) {
            return true;
        }
    }

    false
}

pub struct Filter {
    pub level: Option<RiskLevel>,
    pub domains: Option<Vec<RiskType>>,
}

impl FromStr for Filter {
    type Err = ();

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut tokens = input.split(',').collect::<Vec<&str>>();

        tokens.sort_unstable();
        tokens.dedup();

        let level = tokens
            .iter()
            .filter_map(|t| match *t {
                "crit" => Some(RiskLevel::Critical),
                "critical" => Some(RiskLevel::Critical),
                "high" => Some(RiskLevel::High),
                "hi" => Some(RiskLevel::High),
                "med" => Some(RiskLevel::Medium),
                "medium" => Some(RiskLevel::Medium),
                "info" => Some(RiskLevel::Info),
                "low" => Some(RiskLevel::Low),
                "lo" => Some(RiskLevel::Low),
                _ => None,
            })
            .min();

        let domains = tokens
            .iter()
            .filter_map(|t| match *t {
                "aut" => Some(RiskType::AuthorsRisk),
                "AUT" => Some(RiskType::AuthorsRisk),
                "auth" => Some(RiskType::AuthorsRisk),
                "author" => Some(RiskType::AuthorsRisk),
                "eng" => Some(RiskType::EngineeringRisk),
                "ENG" => Some(RiskType::EngineeringRisk),
                "engineering" => Some(RiskType::EngineeringRisk),
                "code" => Some(RiskType::MaliciousRisk),
                "malicious_code" => Some(RiskType::MaliciousRisk),
                "malicious" => Some(RiskType::MaliciousRisk),
                "mal" => Some(RiskType::MaliciousRisk),
                "MAL" => Some(RiskType::MaliciousRisk),
                "vuln" => Some(RiskType::Vulnerabilities),
                "vulnerability" => Some(RiskType::Vulnerabilities),
                "VLN" => Some(RiskType::Vulnerabilities),
                "vln" => Some(RiskType::Vulnerabilities),
                "license" => Some(RiskType::LicenseRisk),
                "lic" => Some(RiskType::LicenseRisk),
                "LIC" => Some(RiskType::LicenseRisk),
                _ => None,
            })
            .collect::<HashSet<RiskType>>();

        let domains = if domains.is_empty() { None } else { Some(Vec::from_iter(domains)) };

        if level.is_none() && domains.is_none() {
            Err(())
        } else {
            Ok(Filter { level, domains })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Issue;

    #[test]
    fn it_can_parse_filter_levels() {
        let filter_string = "crit,author,med,engineering";

        let filter = Filter::from_str(filter_string).expect("Failed to parse filter string: {}");

        assert_eq!(filter.level, Some(RiskLevel::Medium));

        let filter_string = "foo,crit,author,engineering";

        let filter = Filter::from_str(filter_string).expect("Failed to parse filter string: {}");

        assert_eq!(filter.level, Some(RiskLevel::Critical));
    }

    #[test]
    fn it_can_parse_filter_domains() {
        let filter_string = "crit,author,med,engineering";

        let filter = Filter::from_str(filter_string).expect("Failed to parse filter string: {}");

        let domains = filter.domains.expect("No risk domains parsed from filter string");

        assert_eq!(domains.len(), 2);
        assert!(domains.contains(&RiskType::AuthorsRisk));
        assert!(domains.contains(&RiskType::EngineeringRisk));

        let filter_string = "crit,author,AUT,med,ENG,foo,engineering,VLN";

        let filter = Filter::from_str(filter_string).expect("Failed to parse filter string: {}");

        let domains = filter.domains.expect("No risk domains parsed from filter string");

        assert_eq!(domains.len(), 3);
        assert!(domains.contains(&RiskType::AuthorsRisk));
        assert!(domains.contains(&RiskType::EngineeringRisk));
        assert!(domains.contains(&RiskType::Vulnerabilities));
    }

    #[test]
    fn test_filter_check() {
        let filter_string = "lic";
        let filter = Filter::from_str(filter_string).expect("Failed to parse filter string: {}");

        let issue = r#"{
                    "title": "Commercial license risk in xmlrpc@0.3.0",
                    "description": "license is medium risk",
                    "severity": "medium",
                    "domain": "license"
                    }"#;
        let issue: Issue = serde_json::from_str(issue).unwrap();

        let should_filter = should_filter_issue(&filter, issue.severity, issue.domain);
        assert!(!should_filter);

        let filter_string = "mal";
        let filter = Filter::from_str(filter_string).expect("Failed to parse filter string: {}");
        let should_filter = should_filter_issue(&filter, issue.severity, issue.domain);
        assert!(should_filter);
    }
}
