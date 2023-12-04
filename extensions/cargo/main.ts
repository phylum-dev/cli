import {
  green,
  red,
  yellow,
} from "https://deno.land/std@0.199.0/fmt/colors.ts";
import { crypto } from "https://deno.land/std@0.199.0/crypto/crypto.ts";
import { PhylumApi, PolicyEvaluationResponseRaw } from "phylum";

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

/// Check if there is a cache entry for this lockfile's hash.
async function hasSuccessCached(lockfilePath: string): Promise<boolean> {
  // Check if sucess is cached.
  const hash = await hashFile(lockfilePath);

  try {
    return localStorage.getItem(hash) === "success";
  } catch (_e) {
    return false;
  }
}

/// Create a cache entry for a successful analysis.
async function writeSuccessCache(lockfilePath: string) {
  try {
    const hash = await hashFile(lockfilePath);
    localStorage.setItem(hash, "success");
  } catch (_e) {
    //
  }
}

/// Generate a hash for the contents of a file.
async function hashFile(path: string): Promise<string> {
  const data = await Deno.readFile(path);
  const digest = await crypto.subtle.digest("SHA-512", data);
  return btoa(String.fromCharCode(...new Uint8Array(digest)));
}

// Ensure no arguments are passed before a subcommand.
const firstSubcommand = Deno.args.findIndex((arg) => !arg.startsWith("-"));
if (firstSubcommand > 0) {
  console.error(
    `[${
      red("phylum")
    }] This extension does not support arguments before the first subcommand. Please open an issue if "${
      Deno.args[0]
    }" is not an argument.`,
  );
  Deno.exit(127);
}

// Ignore all commands that shouldn't be intercepted.
if (
  Deno.args.length == 0 ||
  !(
    "build".startsWith(Deno.args[0]) ||
    "check".startsWith(Deno.args[0]) ||
    "test".startsWith(Deno.args[0]) ||
    "clippy".startsWith(Deno.args[0])
  )
) {
  const cmd = new Deno.Command("cargo", { args: Deno.args });
  const status = await cmd.spawn().status;
  Deno.exit(status.code);
}

// Ensure we're in a Cargo root directory.
const root = await findRoot("Cargo.toml");
if (!root) {
  console.error(`[${red("phylum")}] unable to find Cargo project root.`);
  console.error(
    `[${
      red("phylum")
    }] Please change to a Cargo project directory and try again.`,
  );
  Deno.exit(124);
}

// Store initial package manager file state.
const packageLockBackup = new FileBackup(root + "/Cargo.lock");
await packageLockBackup.backup();
const manifestBackup = new FileBackup(root + "/Cargo.toml");
await manifestBackup.backup();

// Analyze new dependencies with phylum.
try {
  await checkDryRun();
} catch (e) {
  await restoreBackup();
  throw e;
}

console.log(`[${green("phylum")}] Installing packages…`);

// Perform the package installation without sandbox.
const cmd = new Deno.Command("cargo", { args: Deno.args });
const installStatus = await cmd.spawn().status;
Deno.exit(installStatus.code);

// Analyze new packages.
async function checkDryRun() {
  console.log(`[${green("phylum")}] Updating lockfile…`);

  // Ensure lockfile is always up to date.
  const status = PhylumApi.runSandboxed({
    cmd: "cargo",
    args: ["generate-lockfile"],
    exceptions: {
      run: ["cargo", "~/.rustup"],
      read: ["./", "~/.cargo", "/etc/passwd"],
      write: ["./", "~/.cargo"],
      net: true,
    },
  });

  // Ensure lockfile update was successful.
  if (!status.success) {
    console.error(`[${red("phylum")}] Lockfile update failed.\n`);
    await abort(status.code ?? 255);
  }
  console.log(`[${green("phylum")}] Lockfile updated successfully.\n`);

  // Check if we have the analysis success cached.
  if (await hasSuccessCached("./Cargo.lock")) {
    console.log(
      `[${green("phylum")}] Analysis success verified from cache.\n`,
    );
    await restoreBackup();
    return;
  }

  console.log(`[${green("phylum")}] Parsing lockfile…`);
  let lockfile;
  try {
    lockfile = await PhylumApi.parseDependencyFile("./Cargo.lock", "cargo");
  } catch (e) {
    console.error(`[${red("phylum")}] Lockfile parsing failed: ${e}.\n`);
    await abort(125);
  }
  console.log(`[${green("phylum")}] Lockfile parsed successfully.\n`);

  // Ensure `checkDryRun` never modifies package manager files,
  // regardless of success.
  await restoreBackup();

  console.log(`[${green("phylum")}] Analyzing packages…`);

  if (!lockfile || lockfile.packages.length === 0) {
    console.log(`[${green("phylum")}] No packages found in lockfile.\n`);
    return;
  }

  const result = await PhylumApi.checkPackagesRaw(lockfile.packages);
  logPackageAnalysisResults(result);

  if (result.is_failure) {
    Deno.exit(127);
  } else if (result.incomplete_packages_count !== 0) {
    Deno.exit(126);
  }

  // Write success to cache.
  await writeSuccessCache("./Cargo.lock");
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

// Write the analysis result status to STDOUT/STDERRR.
function logPackageAnalysisResults(result: PolicyEvaluationResponseRaw) {
  if (result.is_failure) {
    console.error(
      `[${red("phylum")}] Phylum Supply Chain Risk Analysis - FAILURE\n`,
    );
  } else if (result.incomplete_packages_count > 0) {
    console.warn(
      `[${yellow("phylum")}] Phylum Supply Chain Risk Analysis - INCOMPLETE\n`,
    );
  } else {
    console.log(
      `[${green("phylum")}] Phylum Supply Chain Risk Analysis - SUCCESS\n`,
    );
  }

  // Print warning regarding incomplete packages.
  if (result.incomplete_packages_count > 0) {
    // Ensure correct pluralization for incomplete packages.
    let unprocessedText =
      `${result.incomplete_packages_count} unprocessed package`;
    if (result.incomplete_packages_count > 1) {
      unprocessedText += "s";
    }

    const yellowPhylum = yellow("phylum");
    console.warn(
      `[${yellowPhylum}] The analysis contains ${unprocessedText}, preventing a complete risk analysis. Phylum is currently processing these packages and should complete soon. Please wait for up to 30 minutes, then re-run the analysis.\n`,
    );
  }

  // Print policy violations.
  let output = "";
  for (const pkg of result.dependencies) {
    // Skip packages without policy rejections.
    if (pkg.rejections.length === 0) {
      continue;
    }

    output += `[${pkg.registry}] ${pkg.name}@${pkg.version}\n`;

    for (const rejection of pkg.rejections) {
      // Skip suppressed issues.
      if (rejection.suppressed) {
        continue;
      }

      // Format rejection title.
      const domain = `[${rejection.source.domain || "     "}]`;
      const message = `${domain} ${rejection.title}`;

      // Color rejection based on severity.
      let colored;
      if (
        rejection.source.severity === "low" ||
        rejection.source.severity === "info"
      ) {
        colored = green(message);
      } else if (rejection.source.severity === "medium") {
        colored = yellow(message);
      } else {
        colored = red(message);
      }

      output += ` ${colored}\n`;
    }
  }
  if (output.length !== 0) {
    console.error(output + "\n");
  }

  // Print web URI for the job results.
  if (result.job_link) {
    console.log(
      `You can find the interactive report here:\n ${result.job_link}\n`,
    );
  }
}
