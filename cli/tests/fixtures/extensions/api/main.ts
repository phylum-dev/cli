const lockfile = await Phylum.parseDependencyFile("../tests/fixtures/poetry.lock", "poetry");
console.log(lockfile.packages.length);
