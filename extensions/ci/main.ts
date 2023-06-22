import { PhylumApi } from "phylum";

// Ensure required arguments are present.
const args = Deno.args.slice(0);
if (args.length < 4) {
  console.error(
    "Usage: phylum ci <PROJECT> [--group <GROUP>] <LABEL> <BASE> <LOCKFILE...>",
  );
  Deno.exit(1);
}

// Find optional groups argument.
let group = undefined;
const groupArgsIndex = args.indexOf("--group");
if (groupArgsIndex != -1) {
  const groupArgs = args.splice(groupArgsIndex, groupArgsIndex + 1);
  group = groupArgs[1];
}

// Parse remaining arguments.
const project = args[0];
const label = args[1];
const base = args[2];
const lockfiles = args.splice(3);

// Parse new lockfiles.
let packages = [];
for (const lockfile of lockfiles) {
  const lockfileDeps = await PhylumApi.parseLockfile(lockfile);
  packages = packages.concat(lockfileDeps.packages);
}

// Deserialize base dependencies.
const baseDepsJson = await Deno.readTextFile(base);
const baseDeps = JSON.parse(baseDepsJson);

// Short-circuit if there are no dependencies.
if (packages.length == 0) {
  console.log("{}");
  Deno.exit(0);
}

// Submit analysis job.
const jobID = await PhylumApi.analyze(
  packages,
  project,
  group,
  label,
);

// Get analysis job results.
const jobStatus = await PhylumApi.getJobStatus(jobID, baseDeps);

// Output results as JSON.
console.log(JSON.stringify(jobStatus));
