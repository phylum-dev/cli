// deno-lint-ignore-file ban-types

/// <reference types="https://raw.githubusercontent.com/denoland/deno/v1.28.3/core/lib.deno_core.d.ts" />

export class PhylumApi {
  /**
   * Analyze dependencies in a lockfile.
   *
   * Packages are expected in the following format:
   *
   * ```
   * [
   *   { name: "accepts", version: "1.3.8" },
   *   { name: "ms", version: "2.0.0" },
   *   { name: "negotiator", version: "0.6.3" },
   *   { name: "ms", version: "2.1.3" }
   * ]
   * ```
   *
   * @param package_type - Accepted package types are "npm", "pypi", "maven", "rubygems", "nuget", "cargo", and "golang"
   * @param packages - List of packages to analyze
   * @param project - Project name. If undefined, the `.phylum_project` file will be used
   * @param group - Group name
   *
   * @returns Analyze Job ID, which can later be queried with `getJobStatus`.
   */
  static analyze(
    package_type: string,
    packages: object[],
    project?: string,
    group?: string,
  ): Promise<string> {
    return Deno.core.opAsync(
      "analyze",
      package_type,
      packages,
      project,
      group,
    );
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
  static getUserInfo(): Promise<object> {
    return Deno.core.opAsync("get_user_info");
  }

  /** Get the current short-lived API access token. */
  static getAccessToken(): Promise<string> {
    return Deno.core.opAsync("get_access_token");
  }

  /** Get the long-lived user refresh token. */
  static getRefreshToken(): Promise<string> {
    return Deno.core.opAsync("get_refresh_token");
  }

  /**
   * Get job results.
   *
   * @returns Job analysis results
   *
   * Job analysis results example:
   * ```
   * {
   *   job_id: "de2d74b1-3925-4de9-9b8f-0c7b27f9b3c8",
   *   ecosystem: "npm",
   *   user_id: "0f2a8e3d-9f75-49fa-89c7-718c4f87fc93",
   *   user_email: "",
   *   created_at: 1657106760573,
   *   status: "complete",
   *   score: 1,
   *   pass: true,
   *   msg: "Project met threshold requirements",
   *   action: "none",
   *   num_incomplete: 0,
   *   last_updated: 1657106760573,
   *   project: "02a8dcdd-69bd-469f-8c39-be76c786fd2b",
   *   project_name: "api-docs",
   *   label: "uncategorized",
   *   thresholds: { author: 0, engineering: 0, license: 0, malicious: 0, total: 0, vulnerability: 0 },
   *   packages: [
   *     {
   *       name: "typescript",
   *       version: "4.7.4",
   *       status: "complete",
   *       last_updated: 1657106208802,
   *       license: "Apache-2.0",
   *       package_score: 1,
   *       num_dependencies: 0,
   *       num_vulnerabilities: 0,
   *       type: "npm",
   *       riskVectors: {
   *         author: 1,
   *         vulnerabilities: 1,
   *         total: 1,
   *         engineering: 1,
   *         malicious_code: 1,
   *         license: 1
   *       },
   *       dependencies: {},
   *       issues: []
   *     }
   *   ]
   * }
   * ```
   */
  static getJobStatus(jobId: string): Promise<object> {
    return Deno.core.opAsync("get_job_status", jobId);
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
  static getCurrentProject(): object | null {
    return Deno.core.ops.get_current_project();
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
  static getGroups(): Promise<object> {
    return Deno.core.opAsync("get_groups");
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
  static getProjects(group?: string): Promise<object[]> {
    return Deno.core.opAsync("get_projects", group);
  }

  /**
   * Create a project.
   *
   * @return Project ID and status indication
   */
  static createProject(
    name: string,
    group?: string,
  ): Promise<{ id: string; status: "created" | "existed" }> {
    return Deno.core.opAsync("create_project", name, group);
  }

  /**
   * Delete a project.
   *
   * Throws an error if unsuccessful.
   */
  static deleteProject(name: string, group?: string): Promise<void> {
    return Deno.core.opAsync("delete_project", name, group);
  }

  /**
   * Get analysis results for a single package.
   *
   * This will not start a new package analysis, but only retrieve previous
   * analysis results.
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
  ): Promise<object> {
    return Deno.core.opAsync(
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
   *   package_type: "npm",
   *   packages: [
   *     { name: "accepts", version: "1.3.8" },
   *     { name: "ms", version: "2.0.0" },
   *     { name: "negotiator", version: "0.6.3" },
   *     { name: "ms", version: "2.1.3" }
   *   ]
   * }
   * ```
   */
  static parseLockfile(
    lockfile: string,
    lockfileType?: string,
  ): Promise<object> {
    return Deno.core.opAsync("parse_lockfile", lockfile, lockfileType);
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
  static runSandboxed(process: object): object {
    return Deno.core.ops.run_sandboxed(process);
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
  static permissions(): object {
    return Deno.core.ops.op_permissions();
  }
}
