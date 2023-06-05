//! Types for the vulnerability reachability API.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

/// Identified import vulnerability.
#[derive(Serialize, Deserialize, Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug)]
pub struct Vulnerability {
    pub name: String,
    pub summary: String,
    /// Array storing the reachability path through each affected dependency.
    ///
    /// # Example:
    ///
    /// ```ignore
    /// // Packages reduced to their name for brevity.
    /// [
    ///     [server, http, vulnerable],
    ///     [client, http, vulnerable],
    /// ]
    /// ```
    pub vulnerable_dependencies: Vec<Vec<Package>>,
}

/// Dependency package.
#[derive(Serialize, Deserialize, Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug)]
pub struct Package {
    pub name: String,
    pub version: String,
    /// Path taken through this dependency to reach the next vulnerable node.
    pub path: Vec<Callsite>,
}

/// Import usage location.
#[derive(Serialize, Deserialize, Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug)]
pub struct Callsite {
    pub file: String,
    pub start: (usize, usize),
    pub end: (usize, usize),
    pub text: String,
}

/// A reachability analysis job.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct Job {
    /// Job ID for the Phylum issue analysis.
    pub analysis_job_id: String,
    /// The list of transitive dependencies for the user's project.
    pub dependencies: HashSet<JobPackage>,
    /// The list of packages directly imported by the user.
    pub imported_packages: HashSet<String>,
}

/// A globally unique package.
#[derive(Serialize, Deserialize, Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug)]
pub struct JobPackage {
    pub name: String,
    pub version: String,
    pub ecosystem: String,
}
