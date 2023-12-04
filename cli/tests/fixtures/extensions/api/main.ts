import { PhylumApi } from "phylum";

const lockfile = await PhylumApi.parseDependencyFile("../tests/fixtures/poetry.lock", "poetry");
console.log(lockfile.packages.length);
