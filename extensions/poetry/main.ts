import { PhylumApi } from "phylum"
import { red, green, yellow } from "https://deno.land/std@0.150.0/fmt/colors.ts";

class FileBackup {
  readonly fileName: string
  readonly fileContent: string | null

  constructor(fileName: string) {
    this.fileName = fileName
    this.fileContent = null
  }

  async backup() {
    try {
      this.fileContent = await Deno.readTextFile(this.fileName)
    } catch (e) { }
  }

  async restoreOrDelete() {
    try {
      if (this.fileContent != null) {
        await Deno.writeTextFile(this.fileName, this.fileContent)
      } else {
        await Deno.remove(this.fileName)
      }
    } catch (e) { }
  }
}

// Parse the output of `poetry` subcommands that support the `--dry-run` flag,
// and submit the packages.
//
// Add the `-vvv` flags to get a detailed report of the dependency resolution
// process. This way, we can track the actual lockfile changes, as specifying
// `--dry-run` only would not output anything in combination with flags such
// as `--lock` that do not perform the actual operations.
async function poetryCheckDryRun(subcommand: string, args: string[]): number {
  try {
    await Deno.stat('pyproject.toml')
  } catch (e) {
    console.error(`\`pyproject.toml\` was not found in the current directory.`)
    console.error(`Please move to the Poetry project's top level directory and try again.`)
    return 127
  }

  // Read and backup the current poetry lockfile contents.
  const lockfileBackup = new FileBackup('poetry.lock')
  const manifestBackup = new FileBackup('pyproject.toml')

  await lockfileBackup.backup()
  await manifestBackup.backup()

  let process = Deno.run({
    cmd: ['poetry', subcommand, '-vvv', '-n', '--dry-run', ...args.map(s => s.toString())],
    stdout: 'piped',
    stderr: 'piped',
  })

  await process.status()

  const lockfileData = await PhylumApi.parseLockfile('./poetry.lock', 'poetry')

  // If it existed before, restore the previous contents of the lockfile;
  // otherwise, delete it. This is a workaround to the fact that in poetry
  // 1.1.x, the `--dry-run` argument does not prevent the lockfile from 
  // being modified. This is not fixed as of poetry 1.1.14.
  // Prudently, do the same for the manifest (pyproject.toml).
  //
  // See: https://github.com/python-poetry/poetry/pull/5718
  await lockfileBackup.restoreOrDelete()
  await manifestBackup.restoreOrDelete()

  const jobId = await PhylumApi.analyze(lockfileData['package_type'], lockfileData['packages'])
  const jobStatus = await PhylumApi.getJobStatus(jobId)

  if (jobStatus.pass && jobStatus.status === "complete") {
    console.log(`[${green("phylum")}] All packages pass project thresholds.\n`)
    return 0
  } else if (jobStatus.pass) {
    console.warn(`[${yellow("phylum")}] Unknown packages were submitted for analysis, please check again later.\n`)
    return 126
  } else {
    console.error(`[${red("phylum")}] The operation caused a threshold failure.\n`)
    return 127
  }
}

// If the subcommand modifies the lockfile, process it through Phylum.
if (Deno.args.length >= 1 && ['add', 'update', 'install'].includes(Deno.args[0])) {
  const analysisOutcome = await poetryCheckDryRun(Deno.args[0], Deno.args.slice(1))

  // If the analysis failed, exit with an error.
  if (analysisOutcome !== 0) {
    Deno.exit(analysisOutcome)
  }
}

// If the analysis outcome is positive, or no analysis was performed, yield
// control to `poetry` with the arguments originally passed in, and exit with
// its return code.
let status = await Deno.run({
  cmd: ['poetry', ...Deno.args],
}).status()

Deno.exit(status.code)
