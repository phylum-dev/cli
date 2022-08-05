---
title: Extension Example
category: 62c5cb137dbdad00536291a6
---

In this chapter, we'll go over a simple real-world example of what a Phylum CLI
extension might look like.

Our goal is writing an extension which can print out all dependencies with more
than one version present in our lockfile.

The full example looks like this:

```ts
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
const lockfile = await PhylumApi.parseLockfile(Deno.args[0]);

// Group all versions for the same dependency together.
const groupedDeps = groupBy(lockfile.packages, dep => dep.name);

// Reduce each dependency to a list of its versions.
const reducedDeps = mapValues(groupedDeps, deps => deps.map(dep => dep.version));

for (const [dep, versions] of Object.entries(reducedDeps)) {
    // Deduplicate identical versions.
    const distinctVersions = distinct(versions);

    // Print all dependencies with more than one version.
    if (distinctVersions.length > 1) {
        console.log(`${dep}:`, distinctVersions);
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
import { mapValues } from "https://deno.land/std@0.150.0/collections/map_values.ts";
import { distinct } from "https://deno.land/std@0.150.0/collections/distinct.ts";
import { groupBy } from "https://deno.land/std@0.150.0/collections/group_by.ts";
```

These are the Deno API imports. We use version `0.150.0` of [Deno's STD][deno_std]
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
    Deno.exit(1);
}
```

The `Deno.args` variable contains an array with all CLI arguments passed after
our extension name, so for `phylum my-extension one two` that would be `["one",
"two"]`.

Here we make sure that we get exactly one parameter and print a useful help
message to the terminal if no parameter was provided.

The `Deno.exit` function will terminate the extension and return the provided
error code.

```ts
// Parse lockfile using Phylum's API.
const lockfile = await PhylumApi.parseLockfile(Deno.args[0]);
```

The `parseLockfile` method reads the lockfile path passed as an argument and
returns an object containing all dependencies and the package ecosystem. Since
this function is asynchronous, we need to `await` it.

The lockfile object will look something like this:

```text
  packages: [
    { name: "accepts", version: "1.3.8" },
    { name: "array-flatten", version: "1.1.1" },
    { name: "accepts", version: "1.0.0" }
  ],
  package_type: "npm"
}
```

```ts
// Group all versions for the same dependency together.
const groupedDeps = groupBy(lockfile.packages, dep => dep.name);
```

Since our package list contains multiple instances of the same dependency, we
need to group each instance together to find duplicate versions. Deno's
convenient `groupBy` function does this for us automatically and we just need to
tell it which field to group by using `dep => dep.name`.

This will transform our package list into the following:

```text
{
  accepts: [
      { name: "accepts", version: "1.3.8" },
      { name: "accepts", version: "1.0.0" }
  ],
  "array-flatten": [ { name: "array-flatten", version: "1.1.1" } ]
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

```text
{
  accepts: ["1.3.8", "1.0.0"],
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
}
```

With all versions deduplicated, we can finally print out each dependency with
more than one version in our lockfile.

For our example, the output looks like this:

```text
accepts: [ "1.3.8", "1.0.0" ]
```

And that's all the code we need to check for duplicates. Now we can use the
`phylum extension run` subcommand to test the extension without installing it:

```sh
phylum extension run ./duplicates ./package-lock.json
```

This should then print the following error:

```text
Extension error: Uncaught (in promise) Error: Requires read access to "./package-lock.json"
    at async Function.parseLockfile (deno:phylum:201:16)
    at async file:///tmp/duplicates/main.ts:12:14
```

Phylum's extensions are executed in a sandbox with restricted access to
operating system APIs. Since we want to read the lockfile from
`./package-lock.json` with the `parseLockfile` method, we need to request read
access to this file ahead of time. All available permissions are documented in
the [extension manifest documentation].

[extension manifest documentation]: https://docs.phylum.io/docs/extension_manifest#permissions

While it would be possible to request read access to just `./package-lock.json`,
this would only work for `package-lock.json` files defeating the purpose of
passing the lockfile as a parameter. Instead, we request read access to all
files in the working directory:

```toml
[permissions]
read = ["./"]
```

Alternatively if you wanted to allow read access to any file, so lockfiles
outside of the working directory are supported, you could use `read = true`
instead.

Now `phylum extension run` should prompt for these permissions and complete
without any errors if they have been granted. Then we can install and run our
extension:

```sh
phylum extension install ./duplicates
phylum duplicates ./package-lock.json
```
