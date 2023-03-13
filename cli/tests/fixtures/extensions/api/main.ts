import { PhylumApi, ApiVersion } from "https://deno.phylum.io/phylum.ts";

const lockfile = await PhylumApi.parseLockfile("../tests/fixtures/poetry.lock", "poetry");
console.log(lockfile.packages.length);
