import {
  red,
  green,
  yellow,
} from "https://deno.land/std@0.150.0/fmt/colors.ts";
import { PhylumApi } from "phylum";

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
    } catch (e) {}
  }

  async restoreOrDelete() {
    try {
      if (this.fileContent != null) {
        await Deno.writeTextFile(this.fileName, this.fileContent);
      } else {
        await Deno.remove(this.fileName);
      }
    } catch (e) {}
  }
}

// Find project root directory.
async function findRoot(manifest: string): string | undefined {
  let workingDir = Deno.cwd();

  // Traverse up to 32 directories to find the root directory.
  for (var i = 0; i < 32; i++) {
    try {
      // Check if manifest exists at location.
      await Deno.stat(workingDir + "/" + manifest);
      return workingDir;
    } catch (e) {
      // Pop to parent if manifest doesn't exist.
      workingDir += "/..";
    }
  }

  return undefined;
}

// Ignore all commands that shouldn't be intercepted.
if (
  Deno.args.length == 0 ||
  !["add", "install", "up", "dedupe"].includes(Deno.args[0])
) {
  let cmd = await Deno.run({ cmd: ["yarn", ...Deno.args] });
  let status = await cmd.status();
  Deno.exit(status.code);
}

// Ensure we're in a yarn root directory.
const root = await findRoot("package.json");
if (!root) {
  console.error(`[${red("phylum")}] unable to find yarn project root.`);
  console.error(
    `[${red(
      "phylum"
    )}] Please change to a yarn project directory and try again.`
  );
  Deno.exit(125);
}

// Store initial package manager file state.
const packageLockBackup = new FileBackup(root + "/yarn.lock");
await packageLockBackup.backup();
const manifestBackup = new FileBackup(root + "/package.json");
await manifestBackup.backup();

// Analyze new dependencies with phylum before install/update.
await checkDryRun(Deno.args[0], Deno.args.slice(1));

console.log(`[${green("phylum")}] Downloading packages to cache…`);

// Download packages to cache without sandbox.
let status = PhylumApi.runSandboxed({
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
  abort(status.code);
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
    `[${red(
      "phylum"
    )}] This could mean one of your packages attempted to access a restricted resource.`
  );
  console.log(
    `[${red("phylum")}] Do not retry installation without Phylum's extension.`
  );
  console.log(`[${red("phylum")}]`);
  console.log(
    `[${red(
      "phylum"
    )}] Please submit your lockfile to Phylum should this error persist.`
  );

  abort(output.code);
} else {
  console.log(`[${green("phylum")}] Packages built successfully.`);
}

// Analyze new packages.
async function checkDryRun(subcommand: string, args: string[]) {
  console.log(`[${green("phylum")}] Updating lockfile…`);

  let status = PhylumApi.runSandboxed({
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
    abort(status.code);
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

  const jobId = await PhylumApi.analyze("npm", lockfile.packages);
  const jobStatus = await PhylumApi.getJobStatus(jobId);

  if (jobStatus.pass && jobStatus.status === "complete") {
    console.log(`[${green("phylum")}] All packages pass project thresholds.\n`);
  } else if (jobStatus.pass) {
    console.warn(
      `[${yellow(
        "phylum"
      )}] Unknown packages were submitted for analysis, please check again later.\n`
    );
    Deno.exit(126);
  } else {
    console.error(
      `[${red("phylum")}] The operation caused a threshold failure.\n`
    );
    Deno.exit(127);
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
  await manifestBackup.restoreOrDelete();
}
