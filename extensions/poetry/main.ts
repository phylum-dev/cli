import { PhylumApi } from "phylum";
import {
  green,
  red,
  yellow,
} from "https://deno.land/std@0.150.0/fmt/colors.ts";

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

// List with all of poetry's subcommands.
const knownSubcommands = [
  "about",
  "add",
  "build",
  "check",
  "config",
  "export",
  "help",
  "init",
  "install",
  "list",
  "lock",
  "new",
  "publish",
  "remove",
  "run",
  "search",
  "shell",
  "show",
  "update",
  "version",
  "cache",
  "debug",
  "env",
  "self",
  "source",
];

// Ensure the first argument is a known subcommand.
//
// This prevents us from skipping the analysis when an argument is passed before
// the first subcommand (i.e.: `poetry --no-color add package`).
const subcommand = Deno.args[0];
if (Deno.args.length != 0 && !knownSubcommands.includes(subcommand)) {
  console.error(
    `[${
      red("phylum")
    }] This extension does not support arguments before the first subcommand. Please open an issue if "${subcommand}" is not an argument.`,
  );
  Deno.exit(127);
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
  Deno.exit(126);
}

// Analyze new dependencies with phylum before install/update.
await poetryCheckDryRun(Deno.args[0], Deno.args.slice(1));

// Execute install without sandboxing after successful analysis.
const cmd = Deno.run({ cmd: ["poetry", ...Deno.args] });
const status = await cmd.status();
Deno.exit(status.code);

// Analyze new packages.
async function poetryCheckDryRun(
  subcommand: string,
  args: string[],
) {
  const result = PhylumApi.runSandboxed({
    cmd: "poetry",
    args: [subcommand, "-n", "--dry-run", ...args.map((s) => s.toString())],
    exceptions: {
      run: [
        "./",
        "/bin",
        "/usr/bin",
        "~/.pyenv",
        "~/.local/bin/poetry",
        "~/Library/Application Support/pypoetry",
        "~/.local/share/pypoetry",
      ],
      write: [
        "~/.cache/pypoetry",
        "~/Library/Caches/pypoetry",
        "~/.pyenv",
      ],
      read: [
        "./",
        "~/.cache/pypoetry",
        "~/Library/Caches/pypoetry",
        "~/.pyenv",
        "~/Library/Preferences/pypoetry",
        "~/.config/pypoetry",
        "/etc/passwd",
      ],
      net: true,
    },
    stdout: "piped",
  });

  // Ensure dry-run update was successful.
  if (!result.success || result.stdout.length == 0) {
    console.error(`[${red("phylum")}] Failed to determine new packages.\n`);
    return status.code ?? 255;
  }

  // Parse dry-run output to look for new packages.
  const packages = [];
  const lines = result.stdout.split("\n");
  for (const line of lines) {
    const installing_text = "Installing ";
    const installing_index = line.indexOf(installing_text);

    // Filter lines unrelated to new packages.
    if (installing_index === -1) {
      continue;
    }

    // Extract name and version.
    const pkg = line.substring(installing_index + installing_text.length);
    const pkg_split = pkg.split(" ");
    const name = pkg_split[0];
    let version = pkg_split[1];

    // Strip suffix explaining why package install was skipped.
    const colon_index = version.indexOf(":");
    if (colon_index !== -1) {
      version = version.substring(0, colon_index);
    }

    // Ensure what we parsed is in a sensible format.
    if (
      name.length === 0 ||
      version.length === 0 ||
      !version.startsWith("(") ||
      !version.endsWith(")")
    ) {
      console.error(`[${red("phylum")}] Invalid poetry output: ${line}.\n`);
      Deno.exit(125);
    }

    // Remove parenthesis from version.
    version = version.substring(1, version.length - 1);

    packages.push({ name, version });
  }

  // Abort if there's nothing to analyze.
  if (packages.length === 0) {
    console.log(`[${green("phylum")}] No packages found for analysis.\n`);
    return;
  }

  // Run Phylum analysis on the packages.
  const jobId = await PhylumApi.analyze("pypi", packages);
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
    Deno.exit(124);
  } else {
    console.error(
      `[${red("phylum")}] The operation caused a threshold failure.\n`,
    );
    Deno.exit(123);
  }
}
