import {
  green,
  red,
  yellow,
} from "https://deno.land/std@0.150.0/fmt/colors.ts";
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
    "install".startsWith(Deno.args[0]) ||
    "isntall".startsWith(Deno.args[0]) ||
    "add".startsWith(Deno.args[0]) ||
    "update".startsWith(Deno.args[0]) ||
    "udpate".startsWith(Deno.args[0]) ||
    "upgrade".startsWith(Deno.args[0]) ||
    "unlink".startsWith(Deno.args[0]) ||
    "uninstall".startsWith(Deno.args[0]) ||
    "remove".startsWith(Deno.args[0]) ||
    "rm".startsWith(Deno.args[0])
  )
) {
  const cmd = new Deno.Command("npm", { args: Deno.args });
  const status = await cmd.spawn().status;
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
const shrinkwrapBackup = new FileBackup(root + "/npm-shrinkwrap.json");
await shrinkwrapBackup.backup();
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
const cmd = new Deno.Command("npm", {
  args: [...Deno.args, "--ignore-scripts"],
  stdout: "inherit",
  stderr: "inherit",
  stdin: "inherit",
});
const status = await cmd.spawn().status;

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
    write: ["~/.npm/_logs", "./"],
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

  // Ensure lockfile is up to date.
  let status;
  if (
    "install".startsWith(subcommand) ||
    "isntall".startsWith(subcommand) ||
    "add".startsWith(subcommand) ||
    "update".startsWith(subcommand) ||
    "udpate".startsWith(subcommand) ||
    "upgrade".startsWith(subcommand)
  ) {
    // Run the command without installation if a new package is installed.
    const cmd = new Deno.Command("npm", {
      args: [
        subcommand,
        "--package-lock-only",
        "--ignore-scripts",
        ...args,
      ],
      stdout: "inherit",
      stderr: "inherit",
      stdin: "inherit",
    });
    status = await cmd.spawn().status;
  } else {
    // Run just install if no new package is added.
    //
    // This is necessary since `remove` does not have the `package-lock-only`
    // and `ignore-scripts` options.
    const cmd = new Deno.Command("npm", {
      args: [
        "install",
        "--package-lock-only",
        "--ignore-scripts",
      ],
      stdout: "inherit",
      stderr: "inherit",
      stdin: "inherit",
    });
    status = await cmd.spawn().status;
  }

  // Ensure lockfile update was successful.
  if (!status.success) {
    console.error(`[${red("phylum")}] Lockfile update failed.\n`);
    await abort(status.code);
  }

  // Use `npm-shrinkwrap.json` if it is present.
  let lockfilePath = "./package-lock.json";
  try {
    await Deno.stat("./npm-shrinkwrap.json");
    lockfilePath = "./npm-shrinkwrap.json";
  } catch (_e) {
    //
  }

  let lockfile;
  try {
    lockfile = await PhylumApi.parseLockfile(lockfilePath, "npm");
  } catch (_e) {
    console.warn(`[${yellow("phylum")}] No lockfile created.\n`);
    return;
  }

  // Ensure `checkDryRun` never modifies package manager files,
  // regardless of success.
  await restoreBackup();

  console.log(`[${green("phylum")}] Lockfile updated successfully.\n`);
  console.log(`[${green("phylum")}] Analyzing packages…`);

  if (lockfile.packages.length === 0) {
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
