import { PhylumApi, PolicyEvaluationResponseRaw } from "phylum";
import {
  green,
  red,
  yellow,
} from "https://deno.land/std@0.150.0/fmt/colors.ts";

// Ensure no arguments are passed before `add`/`update`/`install` subcommand.
//
// This prevents us from skipping the analysis when an argument is passed before
// the first subcommand (i.e.: `pip --no-color install package`).
const firstSubcommand = Deno.args.findIndex((arg) => !arg.startsWith("-"));
if (firstSubcommand > 0) {
  console.error(
    `[${
      red("phylum")
    }] This extension does not support arguments before the first subcommand. Please open an issue if "${
      Deno.args[0]
    }" is not an argument.`,
  );
  Deno.exit(125);
}

// Ignore all commands that shouldn't be intercepted.
if (Deno.args.length == 0 || Deno.args[0] != "install") {
  const cmd = new Deno.Command("pip3", { args: Deno.args });
  const status = await cmd.spawn().status;
  Deno.exit(status.code);
}

// Ensure the pip version requirements are met.
checkPipVersion();

// Analyze new dependencies with phylum before install/update.
await checkDryRun();

// Perform the package installation.
const installStatus = PhylumApi.runSandboxed({
  cmd: "pip3",
  args: Deno.args,
  exceptions: {
    run: [
      "./",
      "/bin",
      "/usr/bin",
      "/usr/local/bin",
      "/usr/share/pyenv",
      "~/.pyenv",
    ],
    write: [
      "./",
      "~/Library/Caches",
      "~/Library/Python",
      "~/.cache",
      "~/.local",
      "~/.pyenv",
      "/tmp",
    ],
    read: [
      "~/Library/Caches",
      "~/Library/Python",
      "~/.cache",
      "~/.local",
      "/tmp",
      "/etc/passwd",
    ],
    net: true,
  },
});
Deno.exit(installStatus.code ?? 255);

// Analyze new packages.
async function checkDryRun() {
  console.log(`[${green("phylum")}] Finding new dependencies…`);

  const status = PhylumApi.runSandboxed({
    cmd: "pip3",
    args: [...Deno.args, "--quiet", "--report", "-", "--dry-run"],
    exceptions: {
      run: [
        "./",
        "/bin",
        "/usr/bin",
        "/usr/local/bin",
        "/usr/share/pyenv",
        "~/.pyenv",
      ],
      write: [
        "./",
        "~/Library/Caches",
        "~/.pyenv",
        "~/.cache",
        "~/.local/lib",
        "/tmp",
      ],
      read: [
        "~/Library/Caches",
        "~/.cache",
        "~/.local/lib",
        "/tmp",
        "/etc/passwd",
      ],
      net: true,
    },
    stdout: "piped",
  });

  // Ensure dry-run was successful.
  if (!status.success) {
    console.error(`[${red("phylum")}] Pip dry-run failed.\n`);
    Deno.exit(status.code ?? 255);
  }

  // Parse dry-run output.
  let packages;
  try {
    packages = parseDryRun(status.stdout);
  } catch (_e) {
    console.warn(`[${yellow("phylum")}] Ignoring non-JSON dry-run output.\n`);
    return;
  }

  console.log(`[${green("phylum")}] Dependency resolution successful.\n`);
  console.log(`[${green("phylum")}] Analyzing packages…`);

  if (packages.length === 0) {
    console.log(`[${green("phylum")}] No new packages found for analysis.\n`);
    return;
  }

  const result = await PhylumApi.checkPackagesRaw(packages);
  logPackageAnalysisResults(result);

  if (result.is_failure) {
    Deno.exit(127);
  } else if (result.incomplete_packages_count !== 0) {
    Deno.exit(126);
  }
}

type Package = {
  name: string;
  version: string;
  type: string;
};

// Ref: https://pip.pypa.io/en/stable/reference/installation-report/
type DryRunReport = {
  version: string;
  install: {
    is_direct: boolean;
    metadata: {
      name: string;
      version: string;
    };
  }[];
};

// Parse the dry-run report of `pip install`.
function parseDryRun(output: string): Package[] {
  // Output package list.
  const packages: Package[] = [];

  // Parse dependency names and versions.
  const report = JSON.parse(output) as DryRunReport;

  if (report.version !== "1") {
    console.error(`[${red("pip")}] Unsupported pip version!`);
    Deno.exit(255);
  }

  const deps = report.install;
  for (const dep of deps) {
    if (!dep.is_direct) {
      packages.push({
        name: dep.metadata.name,
        version: dep.metadata.version,
        type: "pypi",
      });
    } else {
      // Filesystem paths or direct URLs
      console.warn(
        `[${
          yellow("phylum")
        }] Cannot analyze dependency: ${dep.metadata.name}-${dep.metadata.version}`,
      );
    }
  }

  return packages;
}

// Ensure pip version is at least 23.0.0.
function checkPipVersion() {
  const versionStatus = PhylumApi.runSandboxed({
    cmd: "pip3",
    args: ["--version"],
    exceptions: {
      run: [
        "./",
        "/bin",
        "/usr/bin",
        "/usr/local/bin",
        "/usr/share/pyenv",
        "~/.pyenv",
      ],
    },
    stdout: "piped",
  });

  // Ensure command exited without error.
  if (!versionStatus.success) {
    console.error(`[${red("phylum")}] 'pip --version' failed.\n`);
    Deno.exit(versionStatus.code ?? 255);
  }

  // Check major version in STDOUT.
  const versionMatch = /^pip ([0-9]*)/.exec(versionStatus.stdout);
  if (versionMatch === null || !(parseInt(versionMatch[1]) >= 23)) {
    console.error(
      `[${red("phylum")}] Pip version 23.0.0 or higher is required.\n`,
    );
    Deno.exit(128);
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
