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

// Ensure no arguments are passed before a subcommand.
//
// This prevents us from skipping the analysis when an argument is passed before
// the first subcommand (i.e.: `poetry --no-color add package`).
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
  !["add", "update", "install", "remove"].includes(Deno.args[0])
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

// Analyze new dependencies with phylum.
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
  const result = Phylum.runSandboxed({
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
  const checkResult = await Phylum.checkPackagesRaw(packages);
  logPackageAnalysisResults(checkResult);

  if (checkResult.is_failure) {
    Deno.exit(122);
  } else if (checkResult.incomplete_packages_count !== 0) {
    Deno.exit(123);
  }
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
