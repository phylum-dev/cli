import {
  green,
  red,
  yellow,
} from "https://deno.land/std@0.150.0/fmt/colors.ts";
import { PhylumApi } from "phylum";

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
  !(
    "install".startsWith(Deno.args[0]) ||
    "isntall".startsWith(Deno.args[0]) ||
    "update".startsWith(Deno.args[0]) ||
    "udpate".startsWith(Deno.args[0])
  )
) {
  const cmd = Deno.run({ cmd: ["npm", ...Deno.args] });
  const status = await cmd.status();
  Deno.exit(status.code);
}

// Ensure we're in an npm root directory.
const root = await findRoot("package.json");
if (!root) {
  console.error(`[${red("phylum")}] unable to find npm project root.`);
  console.error(
    `[${
      red("phylum")
    }] Please change to a npm project directory and try again.`,
  );
  Deno.exit(125);
}

// Store initial package manager file state.
const packageLockBackup = new FileBackup(root + "/package-lock.json");
await packageLockBackup.backup();
const manifestBackup = new FileBackup(root + "/package.json");
await manifestBackup.backup();

// Analyze new dependencies with phylum before install/update.
try {
  await checkDryRun(Deno.args[0], Deno.args.slice(1));
} catch (e) {
  await restoreBackup();
  throw e;
}

console.log(`[${green("phylum")}] Installing without build scripts…`);

// Install packages without executing build scripts.
const cmd = Deno.run({
  cmd: ["npm", ...Deno.args, "--ignore-scripts"],
  stdout: "inherit",
  stderr: "inherit",
  stdin: "inherit",
});
const status = await cmd.status();

// Ensure install worked. Failure is still "safe" for the user.
if (!status.success) {
  console.error(`[${red("phylum")}] Installing packges failed.\n`);
  await abort(status.code);
} else {
  console.log(`[${green("phylum")}] Packages installed successfully.\n`);
}

console.log(`[${green("phylum")}] Running build scripts inside sandbox…`);

// Run build scripts inside a sandbox.
const output = PhylumApi.runSandboxed({
  cmd: "npm",
  args: ["install"],
  exceptions: {
    write: ["~/.npm/_logs", "./package-lock.json", "./node_modules"],
    read: true,
    run: true,
    net: false,
  },
});

// Failure here could indicate vulnerabilities; report to the user.
if (!output.success) {
  console.log(`[${red("phylum")}] Sandboxed build failed.`);
  console.log(`[${red("phylum")}]`);
  console.log(
    `[${
      red(
        "phylum",
      )
    }] This could mean one of your packages attempted to access a restricted resource.`,
  );
  console.log(
    `[${red("phylum")}] Do not retry installation without Phylum's extension.`,
  );
  console.log(`[${red("phylum")}]`);
  console.log(
    `[${
      red(
        "phylum",
      )
    }] Please submit your lockfile to Phylum should this error persist.`,
  );

  await abort(output.code ?? 255);
} else {
  console.log(`[${green("phylum")}] Packages built successfully.`);
}

// Analyze new packages.
async function checkDryRun(subcommand: string, args: string[]) {
  console.log(`[${green("phylum")}] Updating lockfile…`);

  const cmd = Deno.run({
    cmd: [
      "npm",
      subcommand,
      "--package-lock-only",
      "--ignore-scripts",
      ...args,
    ],
    stdout: "inherit",
    stderr: "inherit",
    stdin: "inherit",
  });
  const status = await cmd.status();

  // Ensure lockfile update was successful.
  if (!status.success) {
    console.error(`[${red("phylum")}] Lockfile update failed.\n`);
    await abort(status.code);
  }

  const lockfile = await PhylumApi.parseLockfile("./package-lock.json", "npm");

  // Ensure `checkDryRun` never modifies package manager files,
  // regardless of success.
  await restoreBackup();

  console.log(`[${green("phylum")}] Lockfile updated successfully.\n`);
  console.log(`[${green("phylum")}] Analyzing packages…`);

  if (lockfile.packages.length === 0) {
    console.log(`[${green("phylum")}] No packages found in lockfile.\n`);
    return;
  }

  const jobId = await PhylumApi.analyze(lockfile.packages);
  const jobStatus = await PhylumApi.getJobStatus(jobId);

  if (!jobStatus.is_failure && jobStatus.incomplete_count == 0) {
    console.log(`[${green("phylum")}] Supply Chain Risk Analysis - SUCCESS\n`);
  } else if (!jobStatus.is_failure) {
    console.warn(
      `[${yellow("phylum")}] Supply Chain Risk Analysis - INCOMPLETE`,
    );
    console.warn(
      `[${
        yellow(
          "phylum",
        )
      }] Unknown packages were submitted for analysis, please check again later.\n`,
    );
    Deno.exit(126);
  } else {
    console.error(
      `[${red("phylum")}] Supply Chain Risk Analysis - FAILURE\n`,
    );
    Deno.exit(127);
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
