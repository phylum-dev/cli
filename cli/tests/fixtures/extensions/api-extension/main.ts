async function analyze(lockfile: string, project?: string, group?: string) {
  return await Deno.core.opAsync("analyze", lockfile, project, group)
}

async function get_user_info() {
  return await Deno.core.opAsync("get_user_info")
}

async function get_access_token(ignore_certs?: bool) {
  return await Deno.core.opAsync("get_access_token" ?? false)
}

async function get_refresh_token() {
  return await Deno.core.opAsync("get_refresh_token")
}

async function get_job_status(job_id?: string) {
  return await Deno.core.opAsync("get_job_status", job_id)
}

async function get_project_details(project_name?: string) {
  return await Deno.core.opAsync("get_project_details", project_name)
}

async function analyze_package(name: string, version: string, ecosystem: string) {
  return await Deno.core.opAsync("analyze_package", name, version, ecosystem)
}

async function parse_lockfile(lockfile: string, lockfile_type: string) {
  return await Deno.core.opAsync("parse_lockfile", lockfile, lockfile_type)
}
