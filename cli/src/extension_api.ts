// TODO: Figure out how to make API async-safe.
// TODO: Always use String in API, since &str can't easily cross JS boundary.
// TODO: Auto-generate interface types?
// TODO: Auto-generate the entire file?
// TODO: Document methods.

export class PhylumApi {
    static async analyze(lockfile: string, project?: string, group?: string): string {
        return await Deno.core.opAsync('analyze', lockfile, project, group);
    }

    static async getUserInfo(): UserInfo {
        return await Deno.core.opAsync('get_user_info');
    }

    static async getAccessToken(): string {
        return await Deno.core.opAsync('get_access_token');
    }

    static async getRefreshToken(): string {
        return await Deno.core.opAsync('get_refresh_token');
    }

    static async getJobStatus(jobId?: string): JobStatusResponse {
        return await Deno.core.opAsync('get_job_status', jobId);
    }

    static async getProjectDetails(projectName?: string): ProjectDetailsResponse {
        return await Deno.core.opAsync('get_project_details', projectName);
    }

    static async getPackageDetails(name: string, version: string, packageType: string): Package {
        return await Deno.core.opAsync('get_package_details', name, version, packageType);
    }
}

export interface UserInfo {
    email: string;
    sub?: string;
    name?: string;
    given_name?: string;
    family_name?: string;
    preferred_username?: string;
    email_verified?: boolean;
}

export interface JobStatus {
    /// The id of the job processing the top level package
    job_id: string;
    /// The language ecosystem
    ecosystem: string;
    /// The id of the user submitting the job
    user_id: string;
    /// The user email
    user_email: string;
    /// The time the job started in epoch seconds
    created_at: number;
    /// The job status
    status: string;
    /// The current score
    score: number;
    /// Whether the job passed the thresholds
    pass: boolean;
    /// The threshold pass status message
    msg: string;
    /// The action to take if the job fails
    action: string;
    /// Dependencies that have not completed processing
    num_incomplete: number;
    /// The last time the job metadata was updated
    last_updated: number;
    /// The id of the project associated with this job
    project: string;
    /// The project name
    project_name: string;
    /// A label associated with this job, most often a branch name
    label?: string;
    /// The currently configured threshholds for this job. If the scores fall
    /// below these thresholds, then the client should undertake the action
    /// spelled out by the action field.
    thresholds: ProjectThresholds;
    /// The packages that are a part of this job
    packages: [PackageStatus];
}

export interface ProjectThresholds {
    author: number;
    engineering: number;
    license: number;
    malicious: number;
    total: number;
    vulnerability: number;
}

export interface PackageStatus {
    name: string;
    version: string;
    status: string;
    last_updated: number;
    license?: string;
    package_score: number;
    num_dependencies: number;
    num_vulnerabilities: number;
    /// The package_type, npm, etc.
    type: string;
    riskVectors: Map<string, number>;
    dependencies: Map<string, string>;
    issues: [Issue];
}

export interface Issue {
    tag?: string;
    id?: string;
    title: string;
    description: string;
    severity: string;
    domain: string;
}

// pub struct ProjectDetailsResponse {
//     /// The project name
//     pub name: String,
//     /// The project id
//     pub id: String,
//     /// The project ecosystem / package type
//     pub ecosystem: String,
//     /// The configured risk cutoff thresholds for the project
//     pub thresholds: ProjectThresholds,
//     /// Most recent analysis job runs
//     pub jobs: Vec<JobDescriptor>,
// }

// pub struct JobDescriptor {
//     pub job_id: JobId,
//     pub project: String,
//     pub label: String,
//     pub num_dependencies: u32,
//     pub score: f64,
//     pub packages: Vec<PackageDescriptor>,
//     pub pass: bool,
//     pub msg: String,
//     pub date: String,
//     pub ecosystem: String,
//     #[serde(default)]
//     pub num_incomplete: u32,
// }

// pub struct PackageDescriptor {
//     pub name: String,
//     pub version: String,
//     #[serde(rename = "type")]
//     pub package_type: PackageType,
// }

// pub struct Package {
//     pub id: String,
//     pub name: String,
//     pub version: String,
//     pub registry: String,
//     pub published_date: Option<String>,
//     pub latest_version: Option<String>,
//     pub versions: Vec<ScoredVersion>,
//     pub description: Option<String>,
//     pub license: Option<String>,
//     pub dep_specs: Vec<PackageSpecifier>,
//     pub dependencies: Option<Vec<Package>>,
//     pub download_count: u32,
//     pub risk_scores: RiskScores,
//     pub total_risk_score_dynamics: Option<Vec<ScoreDynamicsPoint>>,
//     pub issues_details: Vec<Issue>,
//     pub issues: Vec<IssuesListItem>,
//     pub authors: Vec<Author>,
//     pub developer_responsiveness: Option<DeveloperResponsiveness>,
//     pub issue_impacts: IssueImpacts,
//     pub complete: bool,
// }
