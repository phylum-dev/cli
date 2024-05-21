/*
 * The functions in this file are documented in `extensions/phylum.d.ts`.
 */

import {
  analyze as op_analyze,
  api_base_url,
  check_packages,
  check_packages_raw,
  create_project,
  delete_project,
  get_access_token,
  get_current_project,
  get_groups,
  get_job_status,
  get_package_details,
  get_projects,
  get_refresh_token,
  get_user_info,
  op_permissions,
  parse_depfile,
  run_sandboxed,
} from "ext:core/ops";

async function ensureRequestHeaders(init) {
  const headers = init.headers = new Headers(init.headers);

  // Set Authorization header if it is missing.
  if (!headers.has("Authorization")) {
    const token = await getAccessToken();
    headers.set("Authorization", `Bearer ${token}`);
  }

  // Set Content-Type header if it is missing.
  if (init.body && !headers.has("Content-Type")) {
    headers.set("Content-Type", "application/json");
  }
}

export async function fetch(
  apiVersion,
  endpoint,
  init,
) {
  // Ensure header object is initialized.
  const fetchInit = init ?? {};

  await ensureRequestHeaders(fetchInit);

  // Get API base URI without version.
  const baseUrl = await apiBaseUrl();

  // Send fetch request.
  return globalThis.fetch(`${baseUrl}/${apiVersion}${endpoint}`, fetchInit);
}

export async function apiBaseUrl() {
  return new URL(await api_base_url());
}

export function analyze(
  packages,
  project,
  group,
  label,
) {
  return op_analyze(
    packages,
    project,
    group,
    label,
  );
}

export function checkPackagesRaw(
  packages,
) {
  return check_packages_raw(packages);
}

export function checkPackages(packages) {
  return check_packages(packages);
}

export function getUserInfo() {
  return get_user_info();
}

export function getAccessToken() {
  return get_access_token();
}

export function getRefreshToken() {
  return get_refresh_token();
}

export function getJobStatus(
  jobId,
  ignoredPackages,
) {
  return get_job_status(jobId, ignoredPackages);
}

export function getJobStatusRaw(
  jobId,
  ignoredPackages,
) {
  return get_job_status(jobId, ignoredPackages);
}

export function getCurrentProject() {
  return get_current_project();
}

export function getGroups() {
  return get_groups();
}

export function getProjects(group) {
  return get_projects(group);
}

export function createProject(
  name,
  group,
  repository_url,
) {
  return create_project(name, group, repository_url);
}

export function deleteProject(name, group) {
  return delete_project(name, group);
}

export function getPackageDetails(
  name,
  version,
  packageType,
) {
  return get_package_details(
    name,
    version,
    packageType,
  );
}

export function parseDependencyFile(
  depfile,
  depfileType,
  generateLockfiles,
  sandboxGeneration,
) {
  return parse_depfile(
    depfile,
    depfileType,
    generateLockfiles,
    sandboxGeneration,
  );
}

export function runSandboxed(process) {
  return run_sandboxed(process);
}

export function permissions() {
  return op_permissions();
}
