import * as PhylumApi from './phylum-api.ts'

// Extension tests should be ran against an integration context, and not in CI
// at least for the moment, as they require access to an API instance, or a
// well-formed, external mock of it.

// Deno.core.print(JSON.stringify(await PhylumApi.get_user_info()))
// await PhylumApi.get_access_token()
// await PhylumApi.get_refresh_token()
// try {
//   await PhylumApi.get_job_status()
// } catch(e) {
//   Deno.core.print(JSON.stringify(e))
// }
// try {
//   await PhylumApi.get_project_details()
// } catch(e) {
//   Deno.core.print(JSON.stringify(e))
// }
