export class PhylumApi {
    /// Analyze dependencies in a lockfile.
    ///
    /// Returns the Job ID, which can later be queried with `getJobStatus`.
    ///
    /// This expects a `.phylum_project` file to be present if the `project`
    /// parameter is undefined.
    static async analyze(lockfile: string, project?: string, group?: string): string {
        return await Deno.core.opAsync('analyze', lockfile, project, group);
    }

    /// Get info about the logged in user.
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
    static async getJobStatus(jobId: string): object {
        return await Deno.core.opAsync('get_job_status', jobId);
    }

    /// Get project info.
    ///
    /// This expects a `.phylum_project` file to be present if the `project`
    /// parameter is undefined.
    static async getProjectDetails(projectName?: string): object {
        return await Deno.core.opAsync('get_project_details', projectName);
    }

    /// Get analysis results for a single package.
    ///
    /// This will not start a new package analysis, but only retrieve previous
    /// analysis results.
    static async getPackageDetails(name: string, version: string, packageType: string): object {
        return await Deno.core.opAsync('get_package_details', name, version, packageType);
    }

    /// Get dependencies inside a lockfile.
    static async parseLockfile(lockfile: string, lockfileType: string): [object] {
        return await Deno.core.opAsync('parse_lockfile', lockfile, lockfileType);
    }
}
