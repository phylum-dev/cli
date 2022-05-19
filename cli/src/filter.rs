use std::collections::HashSet;
use std::iter::FromIterator;
use std::str::FromStr;

use phylum_types::types::package::{RiskLevel, RiskType};

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
                "code" => Some(RiskType::MaliciousCodeRisk),
                "malicious_code" => Some(RiskType::MaliciousCodeRisk),
                "mal" => Some(RiskType::MaliciousCodeRisk),
                "MAL" => Some(RiskType::MaliciousCodeRisk),
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

        assert_eq!(filter.level, Some(RiskLevel::Medium));

        let filter_string = "foo,crit,author,engineering";

        let filter = Filter::from_str(filter_string).expect("Failed to parse filter string: {}");

        assert_eq!(filter.level, Some(RiskLevel::Critical));
    }

    #[test]
    fn it_can_parse_filter_domains() {
        let filter_string = "crit,author,med,engineering";

        let filter = Filter::from_str(filter_string).expect("Failed to parse filter string: {}");

        let domains = filter
            .domains
            .expect("No risk domains parsed from filter string");

        assert_eq!(domains.len(), 2);
        assert!(domains.contains(&RiskType::AuthorsRisk));
        assert!(domains.contains(&RiskType::EngineeringRisk));

        let filter_string = "crit,author,AUT,med,ENG,foo,engineering,VLN";

        let filter = Filter::from_str(filter_string).expect("Failed to parse filter string: {}");

        let domains = filter
            .domains
            .expect("No risk domains parsed from filter string");

        assert_eq!(domains.len(), 3);
        assert!(domains.contains(&RiskType::AuthorsRisk));
        assert!(domains.contains(&RiskType::EngineeringRisk));
        assert!(domains.contains(&RiskType::Vulnerabilities));
    }
}
