// @ts-ignore Deno[Deno.internal].core is not defined in types
const DenoCore = Deno[Deno.internal].core;

export type Package = {
  name: string;
  version: string;
  type: string;
  lockfile?: string;
};

export type Lockfile = {
  packages: Package[];
  format: string;
};

export type ProcessOutput = {
  stdout: string;
  stderr: string;
  success: boolean;
  signal: number | null;
  code: number | null;
};

export type PolicyEvaluationResponseRaw = {
  is_failure: boolean;
  incomplete_packages_count: number;
  help: string;
  job_link: string | null;
  dependencies: EvaluatedDependencies[];
};

export type EvaluatedDependencies = {
  purl: string;
  registry: string;
  name: string;
  version: string;
  rejections: PolicyRejection[];
};

export type PolicyRejection = {
  title: string;
  suppressed: boolean;
  source: RejectionSource;
};

export type RejectionSource = {
  type: string;
  tag: string | null;
  domain: string | null;
  severity: string | null;
  description: string | null;
  reason: string | null;
};

async function requestHeaders(headersInit?: HeadersInit): Promise<Headers> {
  const headers = new Headers(headersInit);

  // Set Authorization header if it is missing.
  if (!headers.has("Authorization")) {
    const token = await PhylumApi.getAccessToken();
    headers.set("Authorization", `Bearer ${token}`);
  }

  // Set Content-Type header if it is missing.
  if (!headers.has("Content-Type")) {
    headers.set("Content-Type", "application/json");
  }

  return headers;
}

export class PhylumApi {
  /**
   * Send a request to the Phylum REST API.
   *
   * See https://api.staging.phylum.io/api/v0/swagger/index.html for available API endpoints.
   *
   * The `init` parameter matches the `init` parameter of the Deno `fetch` function:
   * https://deno.land/api@latest?s=fetch
   */
  static async fetch(
    apiVersion: ApiVersion | string,
    endpoint: string,
    init?: RequestInit,
  ): Promise<Response> {
    // Ensure header object is initialized.
    const fetchInit = init ?? {};

    fetchInit.headers = await requestHeaders(fetchInit.headers);

    // Get API base URI without version.
    const baseUrl = await PhylumApi.apiBaseUrl();

    // Send fetch request.
    return fetch(`${baseUrl}/${apiVersion}${endpoint}`, fetchInit);
  }

  /**
   * Get the Phylum REST API base URL.
   *
   * This will usually return `https://api.phylum.io/api`.
   */
  static async apiBaseUrl(): Promise<URL> {
    return new URL(await DenoCore.opAsync("api_base_url"));
  }

  /**
   * Analyze dependencies in a lockfile.
   *
   * Packages are expected in the following format:
   *
   * ```
   * [
   *   { name: "accepts", version: "1.3.8", type: "npm", lockfile: "/path/to/lockfile" },
   *   { name: "ms", version: "2.0.0", type: "npm" },
   *   { name: "negotiator", version: "0.6.3", type: "npm" },
   *   { name: "ms", version: "2.1.3", type: "npm" }
   * ]
   * ```
   *
   * Accepted package types are "npm", "pypi", "maven", "rubygems", "nuget", "cargo", and "golang"
   * The `lockfile` is an optional string used to represent the path to the lockfile.
   *
   * @param packages - List of packages to analyze
   * @param project - Project name. If undefined, the `.phylum_project` file will be used
   * @param group - Group name
   * @param label - Analysis label for this job
   *
   * @returns Analyze Job ID, which can later be queried with `getJobStatus`.
   */
  static analyze(
    packages: Package[],
    project?: string,
    group?: string,
    label?: string,
  ): Promise<string> {
    return DenoCore.opAsync(
      "analyze",
      packages,
      project,
      group,
      label,
    );
  }

  /**
   * Check a list of packages against the default policy.
   *
   * @param packages - List of packages to check (see `analyze()` for details)
   *
   * @returns Raw job analysis results (see `getJobStatusRaw()` for details)
   */
  static checkPackagesRaw(
    packages: Package[],
  ): Promise<PolicyEvaluationResponseRaw> {
    return DenoCore.opAsync("check_packages_raw", packages);
  }

  /**
   * Check a list of packages against the default policy.
   *
   * @param packages - List of packages to check (see `analyze()` for details)
   *
   * @returns Job analysis results (see `getJobStatus()` for details)
   */
  static checkPackages(packages: Package[]): Promise<Record<string, unknown>> {
    return DenoCore.opAsync("check_packages", packages);
  }

  /**
   * Get info about the logged in user.
   *
   * @returns User information
   *
   * User information object example:
   * ```
   * {
   *   email: "user@phylum.io",
   *   sub: "af8b5c32-9966-496a-e5ae-9ca9ceb43294",
   *   name: "John Doe",
   *   given_name: "John",
   *   family_name: "Doe",
   *   preferred_username: "JD",
   *   email_verified: true,
   * }
   * ```
   */
  static getUserInfo(): Promise<Record<string, unknown>> {
    return DenoCore.opAsync("get_user_info");
  }

  /** Get the current short-lived API access token. */
  static getAccessToken(): Promise<string> {
    return DenoCore.opAsync("get_access_token");
  }

  /** Get the long-lived user refresh token. */
  static getRefreshToken(): Promise<string> {
    return DenoCore.opAsync("get_refresh_token");
  }

  /**
   * Get job results.
   *
   * @param jobId - ID of the analysis job, see `PhylumApi.analyze`.
   * @param ignoredPackages - List of packages which will be ignored in the report.
   *
   * @returns Job analysis results
   *
   * Job analysis results example:
   * ```
   * {
   *   is_failure: false,
   *   incomplete_count: 0,
   *   report: "# Phylum OSS Supply Chain Risk Analysis - SUCCESS\n\nThe Phylum risk analysis is complete and did not identify any issues.\n\n[View this project in the Phylum UI](https://app.staging.phylum.io/projects/739098bc-c954-4bf6-aa36-692f5483edaa?label=uncategorized)\n",
   *   output: "{\"dependencies\":[],\"errors\":[]}"
   * }
   * ```
   */
  static getJobStatus(
    jobId: string,
    ignoredPackages?: Package[],
  ): Promise<Record<string, unknown>> {
    return DenoCore.opAsync("get_job_status", jobId, ignoredPackages);
  }

  /**
   * Get job results.
   *
   * @param jobId - ID of the analysis job, see `PhylumApi.analyze`.
   * @param ignoredPackages - List of packages which will be ignored in the report.
   *
   * @returns Raw job analysis results
   *
   * Job analysis results example:
   * ```
   * {
   *   is_failure: false,
   *   incomplete_packages_count: 0,
   *   help: "The analysis contains 346 package(s) Phylum has not yet processed,\npreventing a complete risk analysis. Phylum is processing these\npackages currently and should complete soon.\nPlease wait for up to 30 minutes, then re-run the analysis.\n\nThis repository analyzes the risk of new dependencies. An\nadministrator of this repository has set requirements via Phylum policy.\n\nIf you see this comment, one or more dependencies have failed Phylum's risk analysis.",
   *   job_link: "https://phylum.io/projects/62a91e89-65cb-4597-8d4b-6ef119b29c5c",
   *   dependencies: [
   *     purl: "pkg:cargo/birdcage@0.2.1",
   *     registry: "cargo",
   *     name: "birdcage",
   *     version: "0.2.1",
   *     rejections: [
   *       title: "Commercial license risk detected in birdcage@0.2.1",
   *       suppressed: false,
   *       source: {
   *         type: "Issue",
   *         tag: "HL0030",
   *         domain: "license",
   *         severity: "high",
   *         description: "### Overview\nThis package uses the **GPL-3.0-or-later** license, which is a **high** risk level to commercial use.",
   *         reason: "risk level cannot exceed medium"
   *       }
   *     ]
   *   ]
   * }
   * ```
   */
  static getJobStatusRaw(
    jobId: string,
    ignoredPackages?: Package[],
  ): Promise<PolicyEvaluationResponseRaw> {
    return DenoCore.opAsync("get_job_status", jobId, ignoredPackages);
  }

  /**
   * Get currently linked project.
   *
   * @returns Linked project information or null
   *
   * Project information example:
   * ```
   * {
   *   id: "a3a898bc-e954-4ff6-ea36-6a19beefedaa",
   *   name: "phylum",
   *   created_at: "2022-08-18T18:41:46.468054855+02:00",
   *   group_name: null
   * }
   * ```
   */
  static getCurrentProject(): Record<string, unknown> | null {
    return DenoCore.ops.get_current_project();
  }

  /**
   * List the user's groups.
   *
   * @returns Accessible groups
   *
   * Accessible groups example:
   * ```
   * {
   *   groups: [
   *     {
   *       created_at: "2022-05-02T14:25:24.184866508Z",
   *       last_modified: "2022-05-02T14:25:24.599950299Z",
   *       owner_email: "null@phylum.io",
   *       group_name: "phlock",
   *       is_admin: false,
   *       is_owner: true
   *     }
   *   ]
   * }
   * ```
   */
  static getGroups(): Promise<Record<string, unknown>> {
    return DenoCore.opAsync("get_groups");
  }

  /**
   * List the user's projects.
   *
   * @returns Accessible projects
   *
   * Accessible projects example
   * ```
   * [
   *   {
   *     name: "phylum",
   *     id: "5d6eaa97-dff8-dead-a619-bcafffefeef0",
   *     updated_at: "2022-08-16T22:22:14.092474Z",
   *     created_at: "2022-06-24T22:45:47.054472Z",
   *     ecosystem: "npm",
   *     group_name: null
   *   }
   * ]
   * ```
   */
  static getProjects(group?: string): Promise<Record<string, unknown>[]> {
    return DenoCore.opAsync("get_projects", group);
  }

  /**
   * Create a project.
   *
   * @return Project ID and status indication
   */
  static createProject(
    name: string,
    group?: string,
    repository_url?: string,
  ): Promise<{ id: string; status: "Created" | "Exists" }> {
    return DenoCore.opAsync("create_project", name, group, repository_url);
  }

  /**
   * Delete a project.
   *
   * Throws an error if unsuccessful.
   */
  static deleteProject(name: string, group?: string): Promise<void> {
    return DenoCore.opAsync("delete_project", name, group);
  }

  /**
   * Get analysis results for a single package.
   *
   * If the requested package has not yet been analyzed by Phylum, it will
   * automatically be submitted for processing.
   *
   * @returns Package analysis results
   *
   * Package analysis results example:
   * ```
   * {
   *   id: "npm:typescript:4.7.4",
   *   name: "typescript",
   *   version: "4.7.4",
   *   registry: "npm",
   *   publishedDate: "2022-06-17T18:21:36+00:00",
   *   latestVersion: null,
   *   versions: [
   *     { version: "4.5.4", total_risk_score: 1 },
   *     { version: "3.9.7", total_risk_score: 1 },
   *     { version: "4.2.4", total_risk_score: 1 }
   *   ],
   *   description: "TypeScript is a language for application scale JavaScript development",
   *   license: "Apache-2.0",
   *   depSpecs: [],
   *   dependencies: [],
   *   downloadCount: 134637844,
   *   riskScores: {
   *     total: 1,
   *     vulnerability: 1,
   *     malicious_code: 1,
   *     author: 1,
   *     engineering: 1,
   *     license: 1
   *   },
   *   totalRiskScoreDynamics: null,
   *   issuesDetails: [],
   *   issues: [],
   *   authors: [],
   *   developerResponsiveness: {
   *     open_issue_count: 0,
   *     total_issue_count: 0,
   *     open_issue_avg_duration: null,
   *     open_pull_request_count: 0,
   *     total_pull_request_count: 0,
   *     open_pull_request_avg_duration: null
   *   },
   *   issueImpacts: { low: 0, medium: 0, high: 0, critical: 0 },
   *   complete: true
   * }
   * ```
   */
  static getPackageDetails(
    name: string,
    version: string,
    packageType: string,
  ): Promise<Record<string, unknown>> {
    return DenoCore.opAsync(
      "get_package_details",
      name,
      version,
      packageType,
    );
  }

  /**
   * Get dependencies inside a lockfile.
   *
   * @returns Lockfile dependencies
   *
   * Lockfile dependencies example:
   * ```
   * {
   *   format: "npm",
   *   packages: [
   *     { lockfile: "package-lock.json", name: "accepts", version: "1.3.8", type: "npm" },
   *     { lockfile: "package-lock.json", name: "ms", version: "2.0.0", type: "npm" },
   *     { lockfile: "package-lock.json", name: "negotiator", version: "0.6.3", type: "npm" },
   *     { lockfile: "package-lock.json", name: "ms", version: "2.1.3", type: "npm" }
   *   ]
   * }
   * ```
   */
  static parseLockfile(
    lockfile: string,
    lockfileType?: string,
    skipSandbox?: boolean,
  ): Promise<Lockfile> {
    return DenoCore.opAsync(
      "parse_lockfile",
      lockfile,
      lockfileType,
      skipSandbox,
    );
  }

  /**
   * Run a command inside a more restrictive sandbox.
   *
   * While all extensions are already sandboxed, it can be useful to further
   * restrict this execution environment when dealing with external commands
   * that could potentially misbehave. This API allows restricting
   * filesystem and network access for those processes.
   *
   * @param process - The command which shall be executed and its restrictions
   *
   * Process example:
   * ```
   * {
   *   cmd: "ls",
   *   args: ["-lah"],
   *   stdin: "null",
   *   stdout: "piped",
   *   stderr: "inherit",
   *   exceptions: {
   *     read: ["~/"],
   *     write: false,
   *     run: ["ls"],
   *     net: false,
   *     strict: false,
   *   }
   * }
   * ```
   *
   * The `read`/`write`/`run` permissions accept either an array of paths,
   * or a boolean. Paths must either be absolute or start with `~/`.
   *
   * For `run` the executables will be resolved from `$PATH` when they are
   * neither absolute nor start with `~/`.
   *
   * The `net` permission accepts only boolean values.
   *
   * Some exceptions are added by default, to simplify the extension creation
   * process. If you're looking for more granular control, you can set strict
   * to `true` and no exceptions will be added without explicitly specifying
   * them.
   *
   * @return Process status and output
   *
   * Process status and output example:
   * ```
   * {
   *   stdout: "Hello, World!",
   *   stderr: "",
   *   success: true,
   *   code: 0,
   * }
   * ```
   *
   * If the process got killed by a signal, it will contain a `signal` field
   * instead of `code`:
   *
   * ```
   * {
   *   stdout: "",
   *   stderr: "Getting killed by signal...",
   *   success: false,
   *   signal: 31,
   * }
   * ```
   */
  static runSandboxed(process: Record<string, unknown>): ProcessOutput {
    return DenoCore.ops.run_sandboxed(process);
  }

  /**
   * Get the extension's manifest permissions.
   *
   * @returns Extension permissions
   *
   * Permissions object example:
   * ```
   * {
   *   read: ["~/.npm"],
   *   write: ["/tmp"],
   *   run: ["ls", "echo", "npm"],
   *   env: ["HOME", "PATH"],
   *   net: false
   * }
   * ```
   */
  static permissions(): Record<string, unknown> {
    return DenoCore.ops.op_permissions();
  }
}

/** Available Phylum REST API versions. **/
export enum ApiVersion {
  V0 = "v0",
}
