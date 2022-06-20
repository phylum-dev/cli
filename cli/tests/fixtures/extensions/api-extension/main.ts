import { PhylumApi } from "phylum";

const packages = PhylumApi.parseLockfile("./tests/fixtures/poetry.lock", "poetry");
console.log(packages.length);
