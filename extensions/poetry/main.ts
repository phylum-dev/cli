import { PhylumApi } from "phylum"
import { red, green, yellow } from "https://deno.land/std@0.150.0/fmt/colors.ts";

class FileBackup {
  readonly fileName: string;
  readonly fileContent: string | null;

  constructor(fileName: string) {
    this.fileName = fileName;
    this.fileContent = null;
  }

  async backup() {
    try {
      this.fileContent = await Deno.readTextFile(this.fileName);
    } catch (e) { }
  }

  async restoreOrDelete() {
    try {
      if (this.fileContent != null) {
        await Deno.writeTextFile(this.fileName, this.fileContent);
      } else {
        await Deno.remove(this.fileName);
      }
    } catch (e) { }
  }
}

// Ensure we're in a poetry root directory.
try {
  await Deno.stat('pyproject.toml');
} catch (e) {
  console.error(`[${red("phylum")}] \`pyproject.toml\` was not found in the current directory.`);
  console.error(`[${red("phylum")}] Please move to the Poetry project's top level directory and try again.`);
  Deno.exit(125);
}

// Store initial package manager file state.
const packageLockBackup = new FileBackup('poetry.lock');
await packageLockBackup.backup();
const manifestBackup = new FileBackup('pyproject.toml');
await manifestBackup.backup();

// If the subcommand modifies the lockfile, process it through Phylum.
if (Deno.args.length >= 1 && ['add', 'update', 'install'].includes(Deno.args[0])) {
  const analysisOutcome = await poetryCheckDryRun(Deno.args[0], Deno.args.slice(1));

  // If the analysis failed, exit with an error.
  if (analysisOutcome !== 0) {
    Deno.exit(analysisOutcome);
  }
} else {
  let status = await Deno.run({ cmd: ['poetry', ...Deno.args] }).status();
  Deno.exit(status.code);
}

// Analyze new packages.
async function poetryCheckDryRun(subcommand: string, args: string[]): number {
  console.log(`[${green("phylum")}] Updating lockfile…`);

  let status = await Deno.run({
    cmd: ['poetry', subcommand, '-n', '--lock', ...args.map(s => s.toString())],
    stdout: 'piped',
    stderr: 'piped',
  }).status();

  // Ensure dry-run update was successful.
  if (!status.success) {
    console.error(`[${red("phylum")}] Lockfile update failed.\n`);
    abort(status.code);
  }

  const lockfileData = await PhylumApi.parseLockfile('./poetry.lock', 'poetry');

  // Ensure `checkDryRun` never modifies package manager files,
  // regardless of success.
  await restoreBackup();

  console.log(`[${green("phylum")}] Lockfile updated successfully.\n`);
  console.log(`[${green("phylum")}] Analyzing packages…`);

  if (lockfile.packages.length === 0) {
    console.log(`[${green("phylum")}] No packages found in lockfile.\n`)
    return;
  }

  const jobId = await PhylumApi.analyze(lockfileData['package_type'], lockfileData['packages']);
  const jobStatus = await PhylumApi.getJobStatus(jobId);

  if (jobStatus.pass && jobStatus.status === "complete") {
    console.log(`[${green("phylum")}] All packages pass project thresholds.\n`);
    return 0;
  } else if (jobStatus.pass) {
    console.warn(`[${yellow("phylum")}] Unknown packages were submitted for analysis, please check again later.\n`);
    return 126;
  } else {
    console.error(`[${red("phylum")}] The operation caused a threshold failure.\n`);
    return 127;
  }
}

// Abort with specified exit code.
//
// This assumes that execution was not successful and it will automatically
// revert to the last stored package manager files.
async function abort(code) {
    await restoreBackup();
    Deno.exit(code);
}

// Restore package manager files.
async function restoreBackup() {
    await packageLockBackup.restoreOrDelete();
    await packageBackup.restoreOrDelete();
}
