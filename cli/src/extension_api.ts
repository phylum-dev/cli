export class PhylumApi {
    /// Analyze dependencies in a lockfile.
    ///
    /// This expects a `.phylum_project` file to be present if the `project`
    /// parameter is undefined.
    ///
    /// # Parameters
    ///
    /// Accepted package types are "npm", "pypi", "maven", "rubygems", and "nuget".
    ///
    /// Packages are expected in the following format:
    ///
    /// ```
    /// [
    ///   { name: "accepts", version: "1.3.8" },
    ///   { name: "ms", version: "2.0.0" },
    ///   { name: "negotiator", version: "0.6.3" },
    ///   { name: "ms", version: "2.1.3" }
    /// ]
    /// ```
    ///
    /// # Returns
    ///
    /// Analyze Job ID, which can later be queried with `getJobStatus`.
    static async analyze(
        package_type: string,
        packages: [object],
        project?: string,
        group?: string,
    ): string {
        return await Deno.core.opAsync('analyze', package_type, packages, project, group);
    }

    /// Get info about the logged in user.
    ///
    /// # Returns
    ///
    /// Object containing user information:
    ///
    /// ```
    /// {
    ///   email: "user@phylum.io",
    ///   sub: "af8b5c32-9966-496a-e5ae-9ca9ceb43294",
    ///   name: "John Doe",
    ///   given_name: "John",
    ///   family_name: "Doe",
    ///   preferred_username: "JD",
    ///   email_verified: true,
    /// }
    /// ```
    static async getUserInfo(): object {
        return await Deno.core.opAsync('get_user_info');
    }

    /// Get the current short-lived API access token.
    static async getAccessToken(): string {
        return await Deno.core.opAsync('get_access_token');
    }

    /// Get the long-lived user refresh token.
    static async getRefreshToken(): string {
        return await Deno.core.opAsync('get_refresh_token');
    }

    /// Get job results.
    ///
    /// # Returns
    ///
    /// Job analysis results:
    ///
    /// ```
    /// {
    ///   job_id: "de2d74b1-3925-4de9-9b8f-0c7b27f9b3c8",
    ///   ecosystem: "npm",
    ///   user_id: "0f2a8e3d-9f75-49fa-89c7-718c4f87fc93",
    ///   user_email: "",
    ///   created_at: 1657106760573,
    ///   status: "complete",
    ///   score: 1,
    ///   pass: true,
    ///   msg: "Project met threshold requirements",
    ///   action: "none",
    ///   num_incomplete: 0,
    ///   last_updated: 1657106760573,
    ///   project: "02a8dcdd-69bd-469f-8c39-be76c786fd2b",
    ///   project_name: "api-docs",
    ///   label: "uncategorized",
    ///   thresholds: { author: 0, engineering: 0, license: 0, malicious: 0, total: 0, vulnerability: 0 },
    ///   packages: [
    ///     {
    ///       name: "typescript",
    ///       version: "4.7.4",
    ///       status: "complete",
    ///       last_updated: 1657106208802,
    ///       license: "Apache-2.0",
    ///       package_score: 1,
    ///       num_dependencies: 0,
    ///       num_vulnerabilities: 0,
    ///       type: "npm",
    ///       riskVectors: {
    ///         author: 1,
    ///         vulnerabilities: 1,
    ///         total: 1,
    ///         engineering: 1,
    ///         malicious_code: 1,
    ///         license: 1
    ///       },
    ///       dependencies: {},
    ///       issues: []
    ///     }
    ///   ]
    /// }
    /// ```
    static async getJobStatus(jobId: string): object {
        return await Deno.core.opAsync('get_job_status', jobId);
    }

    /// Get currently linked project.
    ///
    /// # Returns
    ///
    /// Project information:
    ///
    /// ```
    /// {
    ///   id: "a3a898bc-e954-4ff6-ea36-6a19beefedaa",
    ///   name: "phylum",
    ///   created_at: "2022-08-18T18:41:46.468054855+02:00",
    ///   group_name: null
    /// }
    /// ```
    ///
    /// If no project is linked, this will return `null`.
    static getCurrentProject(): object {
        return Deno.core.opSync('get_current_project');
    }

    /// List the user's groups.
    ///
    /// # Returns
    ///
    /// Accessible groups:
    ///
    /// ```
    /// {
    ///   groups: [
    ///     {
    ///       created_at: "2022-05-02T14:25:24.184866508Z",
    ///       last_modified: "2022-05-02T14:25:24.599950299Z",
    ///       owner_email: "null@phylum.io",
    ///       group_name: "phlock",
    ///       is_admin: false,
    ///       is_owner: true
    ///     }
    ///   ]
    /// }
    /// ```
    static async getGroups(): object {
        return Deno.core.opAsync('get_groups');
    }

    /// List the user's projects.
    ///
    /// # Returns
    ///
    /// Accessible projects:
    ///
    /// ```
    /// [
    ///   {
    ///     name: "phylum",
    ///     id: "5d6eaa97-dff8-dead-a619-bcafffefeef0",
    ///     updated_at: "2022-08-16T22:22:14.092474Z",
    ///     created_at: "2022-06-24T22:45:47.054472Z",
    ///     ecosystem: "npm",
    ///     group_name: null
    ///   }
    /// ]
    /// ```
    static async getProjects(group?: string): object {
        return Deno.core.opAsync('get_projects', group);
    }

    /// Get analysis results for a single package.
    ///
    /// This will not start a new package analysis, but only retrieve previous
    /// analysis results.
    ///
    /// # Returns
    ///
    /// Package analysis results:
    ///
    /// ```
    /// {
    ///   id: "npm:typescript:4.7.4",
    ///   name: "typescript",
    ///   version: "4.7.4",
    ///   registry: "npm",
    ///   publishedDate: "2022-06-17T18:21:36+00:00",
    ///   latestVersion: null,
    ///   versions: [
    ///     { version: "4.5.4", total_risk_score: 1 },
    ///     { version: "3.9.7", total_risk_score: 1 },
    ///     { version: "4.2.4", total_risk_score: 1 }
    ///   ],
    ///   description: "TypeScript is a language for application scale JavaScript development",
    ///   license: "Apache-2.0",
    ///   depSpecs: [],
    ///   dependencies: [],
    ///   downloadCount: 134637844,
    ///   riskScores: {
    ///     total: 1,
    ///     vulnerability: 1,
    ///     malicious_code: 1,
    ///     author: 1,
    ///     engineering: 1,
    ///     license: 1
    ///   },
    ///   totalRiskScoreDynamics: null,
    ///   issuesDetails: [],
    ///   issues: [],
    ///   authors: [],
    ///   developerResponsiveness: {
    ///     open_issue_count: 0,
    ///     total_issue_count: 0,
    ///     open_issue_avg_duration: null,
    ///     open_pull_request_count: 0,
    ///     total_pull_request_count: 0,
    ///     open_pull_request_avg_duration: null
    ///   },
    ///   issueImpacts: { low: 0, medium: 0, high: 0, critical: 0 },
    ///   complete: true
    /// }
    /// ```
    static async getPackageDetails(name: string, version: string, packageType: string): object {
        return await Deno.core.opAsync('get_package_details', name, version, packageType);
    }

    /// Get dependencies inside a lockfile.
    ///
    /// # Returns
    ///
    /// List of dependencies:
    ///
    /// ```
    /// {
    ///   package_type: "npm",
    ///   packages: [
    ///     { name: "accepts", version: "1.3.8" },
    ///     { name: "ms", version: "2.0.0" },
    ///     { name: "negotiator", version: "0.6.3" },
    ///     { name: "ms", version: "2.1.3" }
    ///   ]
    /// }
    /// ```
    static async parseLockfile(lockfile: string, lockfileType?: string): object {
        return await Deno.core.opAsync('parse_lockfile', lockfile, lockfileType);
    }
}
