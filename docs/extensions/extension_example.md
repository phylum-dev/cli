---
title: Extension Example
category: 62c5cb137dbdad00536291a6
hidden: true
---

In this chapter, we'll go over a simple real-world example of what a Phylum CLI
extension might look like.

Our goal is writing an extension which can print out all dependencies with more
than one version present in our lockfile.

The full example looks like this:

```ts
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
            console.log(`${dep}: `, distinctVersions);
        }
    }
}
```

Now there's a lot to unpack here, so we'll go through things one by one:

Before we can start writing our extension code, we need to create our new
extension:

```sh
phylum extension new duplicates
```

We can then start writing the extension by replacing `./duplicates/main.ts` with
our example code:

```ts
import { mapValues } from "https://deno.land/std@0.146.0/collections/map_values.ts";
import { distinct } from "https://deno.land/std@0.146.0/collections/distinct.ts";
import { groupBy } from "https://deno.land/std@0.146.0/collections/group_by.ts";
```

These are the Deno API imports. We use version `0.146.0` of [Deno's STD][deno_std]
here and import the required functions by loading them as remote ES modules.
We'll go into more detail on what we need these for later.

[deno_std]: https://deno.land/std

```ts
import { PhylumApi } from "phylum";
```

This is the import for the built-in Phylum API. You'll see this in most Phylum
API extensions and this is where you find all functionality you need from
Phylum's API.

```ts
// Ensure lockfile argument is present.
if (Deno.args.length != 1) {
    console.error("Usage: phylum duplicates <LOCKFILE>");
```

The `Deno.args` variable contains an array with all CLI arguments passed after
our extension name, so for `phylum my-extension one two` that would be `["one",
"two"]`.

Here we make sure that we get exactly one parameter and print a useful help
message to the terminal if no parameter was provided.

```ts
// Use first CLI parameter as our lockfile.
const lockfile = Deno.args[0];
```

Now we just need to store the first element in a properly named variable and we
have access to the file path passed as first argument.

```ts
// Parse lockfile using Phylum's API.
const deps = await PhylumApi.parseLockfile(lockfile);
```

The `parseLockfile` method reads the lockfile path passed as an argument and
returns a list with all dependencies and their respective versions. Since this
function is asynchronous, we need to `await` it.

The list of packages will look something like this:

```json
[
  { name: "accepts", version: "1.3.8", type: "npm" },
  { name: "array-flatten", version: "1.1.1", type: "npm" },
  { name: "accepts", version: "1.0.0", type: "npm" }
]
```

```ts
// Group all versions for the same dependency together.
const groupedDeps = groupBy(deps, dep => dep.name);
```

Since our package list contains multiple instances of the same dependency, we
need to group each instance together to find duplicate versions. Deno's
convenient `groupBy` function does this for us automatically and we just need to
tell it which field to group by using `dep => dep.name`.

This will transform our package list into the following:

```json
{
  "accepts": [
      { name: "accepts", version: "1.3.8", type: "npm" },
      { name: "accepts", version: "1.0.0", type: "npm" }
  ],
  "array-flatten": [ { name: "array-flatten", version: "1.1.1", type: "npm" } ]
}
```

```ts
// Reduce each dependency to a list of its versions.
const reducedDeps = mapValues(groupedDeps, deps => deps.map(dep => dep.version));
```

Since our dependency structure now contains useless information like `name` and
`type`, we map each of these grouped values to contain only the version numbers
for each dependency.

This results in a simple array with all dependencies and their versions:

```json
{
  "accepts": ["1.3.8", "1.0.0"],
  "array-flatten": ["1.1.1"]
}
```

```ts
for (const [dep, versions] of Object.entries(reducedDeps)) {
```

Since we now have an object containing all dependencies and the required
versions, we can iterate over all fields in this object to check the number of
versions it has.

```ts
// Deduplicate identical versions.
const distinctVersions = distinct(versions);
```

But before we can check the versions themselves, we need to make sure all the
versions are actually unique. Some lockfiles might specify the same version
multiple times, so we need to ensure we filter duplicate versions.

```ts
// Print all dependencies with more than one version.
if (distinctVersions.length > 1) {
    console.log(`${dep}: `, distinctVersions);
}
```

With all versions deduplicated, we can finally print out each dependency with
more than one version in our lockfile.

For our example, the output looks like this:

```console
accepts: [ "1.3.8", "1.0.0" ]
```

And that's all the code we need to check for duplicates. Now we only need to
install it and we can use it for any lockfile we encounter in the future:

```sh
phylum extension install ./duplicates
phylum duplicates ./package-lock.json
```

Currently none of our code does any external I/O, we're completely contained
within the Deno sandbox. Let's change that. Instead of the `console.log`, we'll
now write our output to a file instead:

```ts
await Deno.writeTextFile(
    "./duplicates.txt",
    `${dep}: ${distinctVersions}\n`,
    { append: true }
);
```

When replacing the `console.log` with this function call and executing our
extension, you'll run into the following error:

```console
❗ Error: Execution failed caused by: Error: Requires write access to "./duplicates.txt"
```

This is exactly what should have happened, since Deno's sandbox doesn't allow us
to interact with the outside world unless we've been granted permission to do
so. To request permissions, you'll have to edit the `PhylumExt.toml` manifest
and add the following:

```toml
[permissions]
write = ["./duplicates.txt"]
```

With the permissions added, you'll get prompted during install if you want to
accept the requested permissions. Once you do, the file will be written to
properly during execution.
