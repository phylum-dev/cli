import { PhylumApi } from "phylum"
import { parse } from "https://deno.land/std@0.148.0/flags/mod.ts"
import { PackageSpec, parseDryRun } from "./functions.ts"

// Add packages to the lockfile, and upload it to Phylum for analysis.
// Return `true` if the packages pass the project's thresholds.
// Return `false` if the packages don't pass the thresholds, or the processing
// job is not finished.
async function poetryCheck(args: string[]): boolean {
  console.log("Attempting to add a dependency...")

  let process = Deno.run({ cmd: ["poetry", "add", '--lock', ...args] })
  await process.status()
  await process.close()

  console.log("Analyzing packages...")

  const jobId = await PhylumApi.analyze("./poetry.lock")
  const jobStatus = await PhylumApi.getJobStatus(jobId)

  if (jobStatus.pass && jobStatus.status === "complete") {
    console.log("All packages pass project thresholds.")
    return true
  } else if (jobStatus.pass) {
    console.warn("Unknown packages were submitted for analysis, please check again later.")
    return true
  } else {
    console.error('The operation caused a threshold failure.')
    return false
  }
}

// If the subcommand is `add`, update the lockfile and process it through
// Phylum. Otherwise, pass the arguments through to `poetry`.
if (Deno.args.length >= 1 && Deno.args[0] === 'add') {
  // Skip the `add` string. Pass the rest of the arguments as-is.
  const addArgs = Deno.args.slice(1).map(s => s.toString())
  const analysisOutcome = await poetryCheck(addArgs)

  // If the analysis failed, exit with an error.
  if (!analysisOutcome) {
    Deno.exit(1)
  }
} else if (Deno.args.length >= 1 && Deno.args[0] == 'update') {
} else {
  let status = await Deno.run({
    cmd: ['poetry', ...Deno.args],
  }).status()

  Deno.exit(status.code)
}
