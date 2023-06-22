import { PhylumApi } from "phylum";

// Ensure required arguments are present.
if (Deno.args.length < 4 || Deno.args.length > 5) {
  console.error(
    "Usage: phylum ci <PROJECT> [--group <GROUP>] <LABEL> <BASE> <LOCKFILE>",
  );
  Deno.exit(1);
}

// Find optional groups argument.
let group = undefined;
let groupArgsIndex = Deno.args.indexOf("--group");
if (groupArgsIndex != -1) {
  const groupArgs = Deno.args.splice(groupArgsIndex, groupArgsIndex);
  group = groupArgs[1];
}

// Parse remaining arguments.
const project = Deno.args[0];
const label = Deno.args[1];
const base = Deno.args[2];
const lockfile = Deno.args[3];

// Parse new lockfile.
const lockfileDeps = await PhylumApi.parseLockfile(lockfile);

// Deserialize base dependencies.
const baseDepsJson = await Deno.readTextFile(base);
const baseDeps = JSON.parse(baseDepsJson);

// Submit analysis job.
const jobID = await PhylumApi.analyze(
  lockfileDeps.packages,
  project,
  group,
  label,
);

// Get analysis job results.
const jobStatus = await PhylumApi.getJobStatus(jobID, baseDeps);

// Output results as JSON.
console.log(JSON.stringify(jobStatus));
