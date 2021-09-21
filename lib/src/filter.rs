use std::collections::HashSet;
use std::iter::FromIterator;
use std::str::FromStr;

use crate::types::{RiskDomain, RiskLevel};

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
                "crit" => Some(RiskLevel::Crit),
                "critical" => Some(RiskLevel::Crit),
                "high" => Some(RiskLevel::High),
                "hi" => Some(RiskLevel::High),
                "med" => Some(RiskLevel::Med),
                "medium" => Some(RiskLevel::Med),
                "info" => Some(RiskLevel::Info),
                "low" => Some(RiskLevel::Low),
                "lo" => Some(RiskLevel::Low),
                _ => None,
            })
            .min();

        let domains = tokens
            .iter()
            .filter_map(|t| match *t {
                "AUT" => Some(RiskDomain::AuthorRisk),
                "auth" => Some(RiskDomain::AuthorRisk),
                "author" => Some(RiskDomain::AuthorRisk),
                "eng" => Some(RiskDomain::EngineeringRisk),
                "ENG" => Some(RiskDomain::EngineeringRisk),
                "engineering" => Some(RiskDomain::EngineeringRisk),
                "code" => Some(RiskDomain::MaliciousCode),
                "malicious_code" => Some(RiskDomain::MaliciousCode),
                "MAL" => Some(RiskDomain::MaliciousCode),
                "vuln" => Some(RiskDomain::Vulnerabilities),
                "vulnerability" => Some(RiskDomain::Vulnerabilities),
                "VLN" => Some(RiskDomain::Vulnerabilities),
                "license" => Some(RiskDomain::LicenseRisk),
                "LIC" => Some(RiskDomain::LicenseRisk),
                _ => None,
            })
            .collect::<HashSet<RiskDomain>>();

        let domains = if domains.is_empty() {
            None
        } else {
            Some(Vec::from_iter(domains))
        };

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

    #[test]
    fn it_can_parse_filter_levels() {
        let filter_string = "crit,author,med,engineering";

        let filter = Filter::from_str(filter_string).expect("Failed to parse filter string: {}");

        assert_eq!(filter.level, Some(RiskLevel::Med));

        let filter_string = "foo,crit,author,engineering";

        let filter = Filter::from_str(filter_string).expect("Failed to parse filter string: {}");

        assert_eq!(filter.level, Some(RiskLevel::Crit));
    }

    #[test]
    fn it_can_parse_filter_domains() {
        let filter_string = "crit,author,med,engineering";

        let filter = Filter::from_str(filter_string).expect("Failed to parse filter string: {}");

        let domains = filter
            .domains
            .expect("No risk domains parsed from filter string");

        assert_eq!(domains.len(), 2);
        assert!(domains.contains(&RiskDomain::AuthorRisk));
        assert!(domains.contains(&RiskDomain::EngineeringRisk));

        let filter_string = "crit,author,AUT,med,ENG,foo,engineering,VLN";

        let filter = Filter::from_str(filter_string).expect("Failed to parse filter string: {}");

        let domains = filter
            .domains
            .expect("No risk domains parsed from filter string");

        assert_eq!(domains.len(), 3);
        assert!(domains.contains(&RiskDomain::AuthorRisk));
        assert!(domains.contains(&RiskDomain::EngineeringRisk));
        assert!(domains.contains(&RiskDomain::Vulnerabilities));
    }
}
