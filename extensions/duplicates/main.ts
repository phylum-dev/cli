import { mapValues } from "https://deno.land/std@0.146.0/collections/map_values.ts";
import { distinct } from "https://deno.land/std@0.146.0/collections/distinct.ts";
import { groupBy } from "https://deno.land/std@0.146.0/collections/group_by.ts";

import { PhylumApi } from "phylum";

// Ensure lockfile argument is present.
if (Deno.args.length != 1) {
    console.error("Usage: phylum duplicates <LOCKFILE>");
} else {
    // Use first CLI parameter as our lockfile.
    const lockfile = Deno.args[0];

    // Parse lockfile using Phylum's API.
    const deps = await PhylumApi.parseLockfile(lockfile);

    // Group all versions for the same dependency together.
    const groupedDeps = groupBy(deps, dep => dep.name);

    // Reduce each dependency to a list of its versions.
    const reducedDeps = mapValues(groupedDeps, deps => deps.map(dep => dep.version));

    for (const [dep, versions] of Object.entries(reducedDeps)) {
        // Deduplicate identical versions.
        const distinctVersions = distinct(versions);

        // Print all dependencies with more than one version.
        if (distinctVersions.length > 1) {
            console.log(`${dep}:`, distinctVersions);

            // await Deno.writeTextFile(
            //     "./duplicates.txt",
            //     `${dep}: ${distinctVersions}\n`,
            //     { append: true }
            // );
        }
    }
}
