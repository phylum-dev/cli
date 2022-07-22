import { PhylumApi } from "phylum";

if (Deno.args.length >= 1 && 'install'.startsWith(Deno.args[0])) {
    // Analyze new dependencies with phylum before install.
    await install(Deno.args);
} else {
  let status = await Deno.run({ cmd: ['npm', ...Deno.args] }).status()

  Deno.exit(status.code)
}

// Analyze and install package.
async function install(args: string[]) {
    console.log("Updating package lock…");
    await Deno.run({ cmd: ["npm", "i", "--package-lock-only", ...args] }).status();
    console.log("Package lock updated.\n");

    console.log("Analyzing packages…");
    const jobId = await PhylumApi.analyze("./package-lock.json");
    const jobStatus = await PhylumApi.getJobStatus(jobId);

    if (jobStatus.pass && jobStatus.status === "complete") {
        console.log("All packages pass project thresholds.\n");

        console.log(`Installing '${pkg}'…`);
        await Deno.run({ cmd: ["npm", "i", ...args] }).status();
        console.log("Package install complete.");
    } else if (jobStatus.pass) {
        console.warn("Unknown packages were submitted for analysis, please check again later.");
        Deno.exit(9990);
    } else {
        console.error(`Installing '${pkg}' caused threshold failure.`);
        Deno.exit(9991);
    }
}
