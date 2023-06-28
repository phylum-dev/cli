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

// Ensure no arguments are passed before `add`/`update`/`install` subcommand.
//
// This prevents us from skipping the analysis when an argument is passed before
// the first subcommand (i.e.: `poetry --no-color add package`).
const subcommand = Deno.args[0];
if (
  Deno.args.length != 0 &&
    (Deno.args.includes("add") && subcommand !== "add") ||
  (Deno.args.includes("update") && subcommand !== "update") ||
  (Deno.args.includes("install") && subcommand !== "install")
) {
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
  const cmd = new Deno.Command("poetry", { args: Deno.args });
  const status = await cmd.spawn().status;
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
const cmd = new Deno.Command("poetry", { args: Deno.args });
const status = await cmd.spawn().status;
Deno.exit(status.code);

// Analyze new packages.
async function poetryCheckDryRun(
  subcommand: string,
  args: string[],
) {
  const result = PhylumApi.runSandboxed({
    cmd: "poetry",
    args: [
      subcommand,
      "--no-interaction",
      "--dry-run",
      ...args.map((s) => s.toString()),
    ],
    exceptions: {
      run: [
        "./",
        "/bin",
        "/usr/bin",
        "~/.pyenv",
        "~/.local/bin/poetry",
        "~/Library/Application Support/pypoetry",
        "~/.local/share/pypoetry",
        "~/.local/pipx",
      ],
      write: [
        "./",
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
    Deno.exit(result.code ?? 255);
  }

  // Parse dry-run output to look for new packages.
  const packages = [];
  const lines = result.stdout.split("\n");
  for (const line of lines) {
    const installing_text = "• Installing ";
    const installing_index = line.indexOf(installing_text);

    const updating_text = "• Updating ";
    const updating_index = line.indexOf(updating_text);

    // Strip text before package name.
    let pkg;
    if (installing_index !== -1) {
      pkg = line.substring(installing_index + installing_text.length);
    } else if (updating_index !== -1) {
      pkg = line.substring(updating_index + updating_text.length);
    } else {
      // Filter lines unrelated to new packages.
      continue;
    }

    // Strip suffix explaining why package install was skipped.
    const colon_index = pkg.indexOf(":");
    if (colon_index !== -1) {
      pkg = pkg.substring(0, colon_index);
    }

    // Extract name and version.
    const name = pkg.substring(0, pkg.indexOf(" "));
    let version = pkg.substring(name.length + 1);

    // Ensure the line is in the correct format.
    if (
      name.length === 0 ||
      version.length === 0 ||
      !version.startsWith("(") ||
      !version.endsWith(")")
    ) {
      continue;
    }

    // Remove parenthesis from version.
    version = version.substring(1, version.length - 1);

    // Extract target version from update.
    if (updating_index !== -1) {
      const version_split = version.split(" -> ");
      if (version_split.length !== 2) {
        console.error(
          `[${red("phylum")}] Invalid version update: "${version}".\n`,
        );
        Deno.exit(124);
      }

      version = version_split[1];
    }

    // Truncate URI from versions:
    // "1.2.3 https://github.com/demo/demo.git" -> "1.2.3"
    version = version.split(" ")[0];

    packages.push({ name, version, type: "pypi" });
  }

  // Abort if there's nothing to analyze.
  if (packages.length === 0) {
    console.log(`[${green("phylum")}] No packages found for analysis.\n`);
    return;
  }

  // Run Phylum analysis on the packages.
  const checkResult = await PhylumApi.checkPackages(packages);

  if (!checkResult.is_failure && checkResult.incomplete_count == 0) {
    console.log(`[${green("phylum")}] Supply Chain Risk Analysis - SUCCESS\n`);
  } else if (!checkResult.is_failure) {
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
    Deno.exit(123);
  } else {
    console.error(
      `[${red("phylum")}] Supply Chain Risk Analysis - FAILURE\n`,
    );
    Deno.exit(122);
  }
}
