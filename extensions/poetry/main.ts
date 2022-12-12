import { PhylumApi } from "phylum";
import {
  green,
  red,
  yellow,
} from "https://deno.land/std@0.150.0/fmt/colors.ts";

class FileBackup {
  readonly fileName: string;
  fileContent: string | null;

  constructor(fileName: string) {
    this.fileName = fileName;
    this.fileContent = null;
  }

  async backup() {
    try {
      this.fileContent = await Deno.readTextFile(this.fileName);
    } catch (_e) { /* Do nothing */ }
  }

  async restoreOrDelete() {
    try {
      if (this.fileContent != null) {
        await Deno.writeTextFile(this.fileName, this.fileContent);
      } else {
        await Deno.remove(this.fileName);
      }
    } catch (_e) { /* Do nothing */ }
  }
}

// Find project root directory.
async function findRoot(manifest: string): Promise<string | undefined> {
  let workingDir = Deno.cwd();

  // Traverse up to 32 directories to find the root directory.
  for (let i = 0; i < 32; i++) {
    try {
      // Check if manifest exists at location.
      await Deno.stat(workingDir + "/" + manifest);
      return workingDir;
    } catch (_e) {
      // Pop to parent if manifest doesn't exist.
      workingDir += "/..";
    }
  }

  return undefined;
}

// Ignore all commands that shouldn't be intercepted.
if (
  Deno.args.length == 0 ||
  !["add", "update", "install"].includes(Deno.args[0])
) {
  const cmd = Deno.run({ cmd: ["poetry", ...Deno.args] });
  const status = await cmd.status();
  Deno.exit(status.code);
}

// Ensure we're in a poetry root directory.
// Ensure we're in a yarn root directory.
const root = await findRoot("pyproject.toml");
if (!root) {
  console.error(`[${red("phylum")}] unable to find poetry project root.`);
  console.error(
    `[${
      red(
        "phylum",
      )
    }] Please change to a poetry project directory and try again.`,
  );
  Deno.exit(125);
}

// Store initial package manager file state.
const packageLockBackup = new FileBackup(root + "/poetry.lock");
await packageLockBackup.backup();
const manifestBackup = new FileBackup(root + "/pyproject.toml");
await manifestBackup.backup();

// Analyze new dependencies with phylum before install/update.
let analysisOutcome: number;
try {
  analysisOutcome = await poetryCheckDryRun(Deno.args[0], Deno.args.slice(1));
} catch (e) {
  await restoreBackup();
  throw e;
}

// If the analysis failed, exit with an error.
if (analysisOutcome !== 0) {
  Deno.exit(analysisOutcome);
}

// Analyze new packages.
async function poetryCheckDryRun(subcommand: string, args: string[]): Promise<number> {
  console.log(`[${green("phylum")}] Updating lockfile…`);

  const status = PhylumApi.runSandboxed({
    cmd: "poetry",
    args: [subcommand, "-n", ...args.map((s) => s.toString())],
    exceptions: {
      read: true,
      write: [
        "./",
        "~/.local/share/virtualenv",
        "~/.cache/pypoetry",
        "~/Library/Caches/pypoetry",
      ],
      run: ["poetry", "python", "python3"],
      net: true,
    },
  });

  // Ensure dry-run update was successful.
  if (!status.success) {
    console.error(`[${red("phylum")}] Lockfile update failed.\n`);
    await abort(status.code);
  }

  const lockfileData = await PhylumApi.parseLockfile("./poetry.lock", "poetry");

  // Ensure `checkDryRun` never modifies package manager files,
  // regardless of success.
  await restoreBackup();

  console.log(`[${green("phylum")}] Lockfile updated successfully.\n`);
  console.log(`[${green("phylum")}] Analyzing packages…`);

  if (lockfileData.packages.length === 0) {
    console.log(`[${green("phylum")}] No packages found in lockfile.\n`);
    return 0;
  }

  const jobId = await PhylumApi.analyze(
    lockfileData["package_type"],
    lockfileData["packages"],
  );
  const jobStatus = await PhylumApi.getJobStatus(jobId);

  if (jobStatus.pass && jobStatus.status === "complete") {
    console.log(`[${green("phylum")}] All packages pass project thresholds.\n`);
    return 0;
  } else if (jobStatus.pass) {
    console.warn(
      `[${
        yellow(
          "phylum",
        )
      }] Unknown packages were submitted for analysis, please check again later.\n`,
    );
    return 126;
  } else {
    console.error(
      `[${red("phylum")}] The operation caused a threshold failure.\n`,
    );
    return 127;
  }
}

// Abort with specified exit code.
//
// This assumes that execution was not successful and it will automatically
// revert to the last stored package manager files.
async function abort(code: number) {
  await restoreBackup();
  Deno.exit(code);
}

// Restore package manager files.
async function restoreBackup() {
  await packageLockBackup.restoreOrDelete();
  await manifestBackup.restoreOrDelete();
}
