import { PhylumApi } from "phylum"
import { parse } from "https://deno.land/std@0.148.0/flags/mod.ts"

type PackageSpec = {
  name: string,
  version: string,
}

// Parse a package name and version from a line of a poetry dry run output.
// Returns `null` if no match was found.
function parseDryRunLine(line: string): PackageSpec | null {
  const installingRegexp = /: selecting\s+([^\s]+)\s+\(([^\)]+)\)/

  let matches = line.match(installingRegexp)
  if (matches != null) {
    return {
      name: matches[1],
      version: matches[2],
    }
  } else {
    return null
  }
} 

// Parse all packages from a poetry dry run output.
// Filter out the non-matching lines.
function parseDryRun(output: string): PackageSpec[] {
  function isNotNull<T>(val: T | null): val is T {
    return val !== null
  }

  return output
    .split('\n')
    .map(parseDryRunLine)
    .filter(isNotNull)
}

// Parse the output of `poetry` subcommands that support the `--dry-run` flag,
// and submit the packages.
//
// Add the `-vvv` flags to get a detailed report of the dependency resolution
// process. This way, we can track the actual lockfile changes, as specifying
// `--dry-run` only would not output anything in combination with flags such
// as `--lock` that do not perform the actual operations.
async function poetryCheckDryRun(subcommand: string, args: string[]): boolean {
  console.log("Retrieving changes...")

  let process = Deno.run({
    cmd: ['poetry', subcommand, '-vvv', '-n', '--dry-run', ...args.map(s => s.toString())],
    stdout: 'piped',
    stderr: 'piped',
  })

  await process.status()
  await process.close()

  const output = new TextDecoder().decode(await process.output())
  const packages = parseDryRun(output)

  console.log("Analyzing packages:")
  for (const { name, version } of packages) {
    console.log(`  - ${name} ${version}`)
  }
  console.log()

  const jobId = await PhylumApi.analyze("pypi", packages)
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
if (Deno.args.length >= 1 && ['add', 'update', 'install'].includes(Deno.args[0])) {
  // Skip the `add` string. Pass the rest of the arguments as-is.
  const analysisOutcome = await poetryCheckDryRun(Deno.args[0], Deno.args.slice(1))

  // If the analysis failed, exit with an error.
  if (!analysisOutcome) {
    Deno.exit(1)
  }
}

// If the analysis outcome is positive, or no analysis was performed, yield
// control to `poetry` with the arguments originally passed in, and exit with
// its return code.
let status = await Deno.run({
  cmd: ['poetry', ...Deno.args],
}).status()

Deno.exit(status.code)
