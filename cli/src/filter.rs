use std::collections::HashSet;
use std::iter::FromIterator;
use std::str::FromStr;

use phylum_types::types::job::JobStatusResponse;
use phylum_types::types::package::PackageStatusExtended;

use crate::types::{Package, RiskDomain, RiskLevel};

/// Remove issues based on a filter.
pub trait FilterIssues {
    fn filter(&mut self, filter: &Filter);
}

impl FilterIssues for Package {
    fn filter(&mut self, filter: &Filter) {
        self.issues.retain(|issue| !should_filter_issue(filter, issue.severity, issue.domain));
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
fn should_filter_issue(filter: &Filter, level: RiskLevel, risk_domain: RiskDomain) -> bool {
    if let Some(filter_level) = filter.level {
        if level < filter_level {
            return true;
        }
    }

    if let Some(domains) = &filter.domains {
        if !domains.contains(&risk_domain) {
            return true;
        }
    }

    false
}

pub struct Filter {
    pub level: Option<RiskLevel>,
    pub domains: Option<Vec<RiskDomain>>,
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
                "aut" => Some(RiskDomain::AuthorRisk),
                "AUT" => Some(RiskDomain::AuthorRisk),
                "auth" => Some(RiskDomain::AuthorRisk),
                "author" => Some(RiskDomain::AuthorRisk),
                "eng" => Some(RiskDomain::EngineeringRisk),
                "ENG" => Some(RiskDomain::EngineeringRisk),
                "engineering" => Some(RiskDomain::EngineeringRisk),
                "code" => Some(RiskDomain::Malicious),
                "malicious_code" => Some(RiskDomain::Malicious),
                "malicious" => Some(RiskDomain::Malicious),
                "mal" => Some(RiskDomain::Malicious),
                "MAL" => Some(RiskDomain::Malicious),
                "vuln" => Some(RiskDomain::Vulnerabilities),
                "vulnerability" => Some(RiskDomain::Vulnerabilities),
                "VLN" => Some(RiskDomain::Vulnerabilities),
                "vln" => Some(RiskDomain::Vulnerabilities),
                "license" => Some(RiskDomain::LicenseRisk),
                "lic" => Some(RiskDomain::LicenseRisk),
                "LIC" => Some(RiskDomain::LicenseRisk),
                _ => None,
            })
            .collect::<HashSet<RiskDomain>>();

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
        assert!(domains.contains(&RiskDomain::AuthorRisk));
        assert!(domains.contains(&RiskDomain::EngineeringRisk));

        let filter_string = "crit,author,AUT,med,ENG,foo,engineering,VLN";

        let filter = Filter::from_str(filter_string).expect("Failed to parse filter string: {}");

        let domains = filter.domains.expect("No risk domains parsed from filter string");

        assert_eq!(domains.len(), 3);
        assert!(domains.contains(&RiskDomain::AuthorRisk));
        assert!(domains.contains(&RiskDomain::EngineeringRisk));
        assert!(domains.contains(&RiskDomain::Vulnerabilities));
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
