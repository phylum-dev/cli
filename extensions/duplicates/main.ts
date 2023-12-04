import { mapValues } from "https://deno.land/std@0.150.0/collections/map_values.ts";
import { distinct } from "https://deno.land/std@0.150.0/collections/distinct.ts";
import { groupBy } from "https://deno.land/std@0.150.0/collections/group_by.ts";

import { PhylumApi } from "phylum";

// Ensure lockfile argument is present.
if (Deno.args.length != 1) {
  console.error("Usage: phylum duplicates <LOCKFILE>");
  Deno.exit(1);
}

// Parse lockfile using Phylum's API.
const lockfile = await PhylumApi.parseDependencyFile(Deno.args[0]);

// Group all versions for the same dependency together.
const groupedDeps = groupBy(lockfile.packages, (dep) => dep.name);

// Reduce each dependency to a list of its versions.
const reducedDeps = mapValues(
  groupedDeps,
  (deps) => deps!.map((dep) => dep.version),
);

for (const [dep, versions] of Object.entries(reducedDeps)) {
  // Deduplicate identical versions.
  const distinctVersions = distinct(versions);

  // Print all dependencies with more than one version.
  if (distinctVersions.length > 1) {
    console.log(`${dep}:`, distinctVersions);
  }
}
