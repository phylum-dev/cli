import { PhylumApi } from "phylum";
import {
  green,
  red,
  yellow,
} from "https://deno.land/std@0.150.0/fmt/colors.ts";

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

// Analyze new packages.
async function checkDryRun() {
  console.log(`[${green("phylum")}] Finding new dependencies…`);

  const status = PhylumApi.runSandboxed({
    cmd: "pip3",
    args: [...Deno.args, "--quiet", "--report", "-", "--dry-run"],
    exceptions: {
      run: ["./", "/bin", "/usr/bin", "/usr/local/bin", "~/.pyenv"],
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

  const result = await PhylumApi.packageCheck(packages);

  if (!result.is_failure && result.incomplete_count == 0) {
    console.log(`[${green("phylum")}] Supply Chain Risk Analysis - SUCCESS\n`);
  } else if (!result.is_failure) {
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
