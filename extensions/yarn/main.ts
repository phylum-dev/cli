import {
  green,
  red,
  yellow,
} from "https://deno.land/std@0.150.0/fmt/colors.ts";
import { PhylumApi } from "https://deno.phylum.io/phylum.ts";

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

// List with all of yarn's subcommands.
const knownSubcommands = [
  // Yarn v1+ subcommands.
  "access",
  "add",
  "audit",
  "autoclean",
  "bin",
  "cache",
  "check",
  "config",
  "create",
  "exec",
  "generate-lock-entry",
  "generateLockEntry",
  "global",
  "help",
  "import",
  "info",
  "init",
  "install",
  "licenses",
  "link",
  "list",
  "login",
  "logout",
  "node",
  "outdated",
  "owner",
  "pack",
  "policies",
  "publish",
  "remove",
  "run",
  "tag",
  "team",
  "unlink",
  "unplug",
  "upgrade",
  "upgrade-interactive",
  "upgradeInteractive",
  "version",
  "versions",
  "why",
  "workspace",
  "workspaces",
  // Yarn v2 subcommands.
  "dedupe",
  "dlx",
  "explain",
  "npm",
  "patch",
  "patch-commit",
  "rebuild",
  "set",
  "up",
  "plugin",
  "workspace",
];

// Ensure the first argument is a known subcommand.
//
// This prevents us from skipping the analysis when an argument is passed before
// the first subcommand (i.e.: `yarn --cwd /tmp/project add package`).
const subcommand = Deno.args[0];
if (Deno.args.length != 0 && !knownSubcommands.includes(subcommand)) {
  console.error(
    `[${
      red("phylum")
    }] This extension does not support arguments before the first subcommand. Please open an issue if "${subcommand}" is not an argument.`,
  );
  Deno.exit(125);
}

// Ignore all commands that shouldn't be intercepted.
if (
  Deno.args.length == 0 ||
  !["add", "install", "up", "dedupe"].includes(Deno.args[0])
) {
  const cmd = Deno.run({ cmd: ["yarn", ...Deno.args] });
  const status = await cmd.status();
  Deno.exit(status.code);
}

// Ensure we're in a yarn root directory.
const root = await findRoot("package.json");
if (!root) {
  console.error(`[${red("phylum")}] unable to find yarn project root.`);
  console.error(
    `[${
      red(
        "phylum",
      )
    }] Please change to a yarn project directory and try again.`,
  );
  Deno.exit(125);
}

// Store initial package manager file state.
const packageLockBackup = new FileBackup(root + "/yarn.lock");
await packageLockBackup.backup();
const manifestBackup = new FileBackup(root + "/package.json");
await manifestBackup.backup();

// Analyze new dependencies with phylum before install/update.
try {
  await checkDryRun();
} catch (e) {
  await restoreBackup();
  throw e;
}

console.log(`[${green("phylum")}] Downloading packages to cache…`);

// Download packages to cache without sandbox.
const status = PhylumApi.runSandboxed({
  cmd: "yarn",
  args: [...Deno.args, "--mode=skip-build"],
  exceptions: {
    read: true,
    write: [
      "~/.cache/node",
      "~/.cache/yarn",
      "~/.yarn",
      "./",
      "~/Library/Caches/Yarn",
      "/tmp",
    ],
    run: ["yarn", "node"],
    net: true,
  },
});

// Ensure download worked. Failure is still "safe" for the user.
if (!status.success) {
  console.error(`[${red("phylum")}] Downloading packages to cache failed.\n`);
  await abort(status.code ?? 255);
} else {
  console.log(`[${green("phylum")}] Cache updated successfully.\n`);
}

console.log(`[${green("phylum")}] Building packages inside sandbox…`);

// Run build inside a sandbox.
const output = PhylumApi.runSandboxed({
  cmd: "yarn",
  args: ["install", "--immutable", "--immutable-cache"],
  exceptions: {
    write: ["/tmp", "./.yarn"],
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
async function checkDryRun() {
  console.log(`[${green("phylum")}] Updating lockfile…`);

  const status = PhylumApi.runSandboxed({
    cmd: "yarn",
    args: [...Deno.args, "--mode=update-lockfile"],
    exceptions: {
      read: true,
      write: ["~/.cache/node", "~/.cache/yarn", "~/.yarn", "./", "/tmp"],
      run: ["yarn", "node"],
      net: true,
    },
  });

  // Ensure lockfile update was successful.
  if (!status.success) {
    console.error(`[${red("phylum")}] Lockfile update failed.\n`);
    await abort(status.code ?? 255);
  }

  const lockfile = await PhylumApi.parseLockfile("./yarn.lock", "yarn");

  // Ensure `checkDryRun` never modifies package manager files,
  // regardless of success.
  await restoreBackup();

  console.log(`[${green("phylum")}] Lockfile updated successfully.\n`);
  console.log(`[${green("phylum")}] Analyzing packages…`);

  if (lockfile.packages.length === 0) {
    console.log(`[${green("phylum")}] No packages found in lockfile.\n`);
    return;
  }

  const jobId = await PhylumApi.analyze(undefined, lockfile.packages);
  const jobStatus = await PhylumApi.getJobStatus(jobId);

  if (jobStatus.pass && jobStatus.status === "complete") {
    console.log(`[${green("phylum")}] All packages pass project thresholds.\n`);
  } else if (jobStatus.pass) {
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
      `[${red("phylum")}] The operation caused a threshold failure.\n`,
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
