import { PhylumApi } from "phylum"
import { parse } from "https://deno.land/std/flags/mod.ts"

// Add packages to the lockfile, and upload it to Phylum for analysis.
// Return `true` if the packages pass the project's thresholds.
// Return `false` if the packages don't pass the thresholds, or the processing
// job is not finished.
async function poetryCheck(pkgs: string[]) {
    console.log("Updating package lock…")
    await Deno.run({ cmd: ["poetry", "add", "--lock", ...pkgs] }).status()
    console.log("Package lock updated.\n")

    console.log("Analyzing packages…")
    const jobId = await PhylumApi.analyze("./poetry.lock")
    const jobStatus = await PhylumApi.getJobStatus(jobId)

    if (jobStatus.pass && jobStatus.status === "complete") {
        console.log("All packages pass project thresholds.\n")
        return true
    } else if (jobStatus.pass) {
        console.warn("Unknown packages were submitted for analysis, please check again later.")
        return false
    } else {
        let packages = pkgs.join("', '")
        console.error(`Installing '${packages}' caused threshold failure.`)
        return false
    }
}

// Parse CLI args.
const args = Deno.args;

if (args.length >= 1 && args[0] === 'add') {
  // Parse CLI arguments to extract added package names.
  const parsedArgs = parse(args, {
    string: ['python', 'platform', 'source', 'E'],
    boolean: ['D', 'optional', 'allow-prereleases', 'dry-run', 'lock']
  })

  // Skip the `add` string.
  const packages = parsedArgs._.slice(1).map(s => s.toString())
  const analysisOutcome = await poetryCheck(packages)

  // If the analysis failed, exit with an error.
  if (!analysisOutcome) {
    Deno.exit(1)
  }
}

// If the analysis succeeded or the user passed an unsupported subcommand, pass
// it through to Poetry.
Deno.run({ cmd: ['poetry', ...args] })
