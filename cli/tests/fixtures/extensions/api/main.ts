import { PhylumApi } from "phylum";

const lockfile = await PhylumApi.parseLockfile("../tests/fixtures/poetry.lock", "poetry");
console.log(lockfile.packages.length);
