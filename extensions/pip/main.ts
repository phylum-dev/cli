import {
  green,
  red,
  yellow,
} from "https://deno.land/std@0.150.0/fmt/colors.ts";
import { PhylumApi } from "https://deno.phylum.io/phylum.ts";

// List with all of pip's subcommands.
const knownSubcommands = [
  "install",
  "download",
  "uninstall",
  "freeze",
  "inspect",
  "list",
  "show",
  "check",
  "config",
  "search",
  "cache",
  "index",
  "wheel",
  "hash",
  "completion",
  "debug",
  "help",
];

// Ensure the first argument is a known subcommand.
//
// This prevents us from skipping the analysis when an argument is passed before
// the first subcommand (i.e.: `pip --no-color install package`).
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
if (Deno.args.length == 0 || subcommand != "install") {
  const cmd = Deno.run({ cmd: ["pip3", ...Deno.args] });
  const status = await cmd.status();
  Deno.exit(status.code);
}

// Analyze new dependencies with phylum before install/update.
await checkDryRun();

// Perform the package installation.
const installStatus = PhylumApi.runSandboxed({
  cmd: "pip3",
  args: Deno.args,
  exceptions: {
    run: ["./", "/bin", "/usr/bin", "/usr/local/bin", "~/.pyenv"],
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
      "~/.pyenv",
      "/tmp",
      "/etc/passwd",
      "/etc/apache2/mime.types",
    ],
    net: true,
  },
});
Deno.exit(installStatus.code ?? 255);

type JobStatus = {
  packages: {
    issues: {
      severity: string;
      title: string;
    }[];
  }[];
};

// Get severity level as a number
function severityLevel(s: string): number {
  switch (s) {
    case "low":
      return 1;
    case "medium":
      return 2;
    case "high":
      return 3;
    case "critical":
      return 4;
  }
  return 0;
}

// Logs any identified issues to the screen.
function logIssues(jobStatus: Record<string, unknown>) {
  const issues = (jobStatus as JobStatus).packages.flatMap((p) => p.issues);

  // Sort by severity
  issues.sort((a, b) => severityLevel(b.severity) - severityLevel(a.severity));

  for (const issue of issues) {
    let severity = issue.severity.toLowerCase();

    if (["high", "critical"].indexOf(severity) != -1) {
      severity = red(severity);
    } else if (severity == "medium") {
      severity = yellow(severity);
    } else {
      severity = green(severity);
    }

    console.log(`    [${severity}] ${issue.title}`);
  }
}

// Analyze new packages.
async function checkDryRun() {
  console.log(`[${green("phylum")}] Finding new dependencies…`);

  const status = PhylumApi.runSandboxed({
    cmd: "pip3",
    args: [...Deno.args, "--quiet", "--report", "-", "--dry-run"],
    exceptions: {
      run: ["./", "/bin", "/usr/bin", "/usr/local/bin", "~/.pyenv"],
      write: [
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
        "/etc/apache2/mime.types",
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
  const packages = parseDryRun(status.stdout);

  console.log(`[${green("phylum")}] Dependency resolution successful.\n`);
  console.log(`[${green("phylum")}] Analyzing packages…`);

  if (packages.length === 0) {
    console.log(`[${green("phylum")}] No new packages found for analysis.\n`);
    return;
  }

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
    Deno.exit(126);
  } else {
    console.error(
      `[${red("phylum")}] The operation caused a threshold failure.\n`,
    );

    logIssues(jobStatus);
    Deno.exit(127);
  }
}

type Package = {
  name: string;
  version: string;
  package_type: string;
};

// Ref: https://pip.pypa.io/en/stable/reference/installation-report/
type DryRunReport = {
  version: string;
  install: {
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
    packages.push({
      name: dep.metadata.name,
      version: dep.metadata.version,
      package_type: "pypi",
    });
  }

  return packages;
}
