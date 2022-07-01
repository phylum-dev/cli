import { PhylumApi } from "phylum";

// Parse CLI args.
const args = Deno.args;
if (args.length != 2 || args[0] !== "add") {
    console.error("Usage: phylum yarn add <PKG>");
} else {
    await install(args[1]);
}

// Analyze and install package.
async function install(pkg: string) {
    console.log("Updating package lock…");
    await Deno.run({ cmd: ["yarn", "add", "--mode=update-lockfile", pkg] }).status();
    console.log("Package lock updated.\n");

    console.log("Analyzing packages…");
    const jobId = await PhylumApi.analyze("./yarn.lock");
    const jobStatus = await PhylumApi.getJobStatus(jobId);

    if (jobStatus.pass && jobStatus.status === "complete") {
        console.log("All packages pass project thresholds.\n");

        console.log(`Installing '${pkg}'…`);
        await Deno.run({ cmd: ["yarn", "add", pkg] }).status();
        console.log("Package install complete.");
    } else if (jobStatus.pass) {
        console.warn("Unknown packages were submitted for analysis, please check again later.");
    } else {
        console.error(`Installing '${pkg}' caused threshold failure.`);
    }
}
