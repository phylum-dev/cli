import { PhylumApi } from "phylum";

const packages = await PhylumApi.parseLockfile("./tests/fixtures/poetry.lock", "poetry");
console.log(packages.length);
