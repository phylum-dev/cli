import { red, green, yellow } from 'https://deno.land/std@0.150.0/fmt/colors.ts';
import { PhylumApi } from 'phylum';

class FileBackup {
  readonly fileName: string
  readonly fileContent: string | null

  constructor(fileName: string) {
    this.fileName = fileName
    this.fileContent = null
  }

  async backup() {
    try {
      this.fileContent = await Deno.readTextFile(this.fileName)
    } catch (e) {}
  }

  async restoreOrDelete() {
    try {
      if (this.fileContent != null) {
        await Deno.writeTextFile(this.fileName, this.fileContent)
      } else {
        await Deno.remove(this.fileName)
      }
    } catch (e) {}
  }
}

// Analyze new packages.
async function checkDryRun(subcommand: string, args: string[]) {
    try {
        await Deno.stat('package.json');
    } catch (e) {
        console.error(`[${red("phylum")}] \`package.json\` was not found in the current directory.`);
        console.error(`[${red("phylum")}] Please move to the yarn project's top level directory and try again.`);
        return 125;
    }

    // Backup package/lock files.
    const packageLockBackup = new FileBackup('./yarn.lock');
    await packageLockBackup.backup();
    const packageBackup = new FileBackup('./package.json');
    await packageBackup.backup();

    await Deno.run({
        cmd: ['yarn', subcommand, '--mode=update-lockfile', ...args],
        stdout: 'piped',
        stderr: 'piped',
    }).status();

    const lockfile = await PhylumApi.parseLockfile('./yarn.lock', 'yarn');

    // Restore package/lock files.
    await packageLockBackup.restoreOrDelete();
    await packageBackup.restoreOrDelete();

    console.log(`[${green("phylum")}] Analyzing packages…`);

    if (lockfile.packages.length === 0) {
        console.log(`[${green("phylum")}] No packages found in lockfile.\n`)
        return;
    }

    const jobId = await PhylumApi.analyze('npm', lockfile.packages);
    const jobStatus = await PhylumApi.getJobStatus(jobId);

    if (jobStatus.pass && jobStatus.status === 'complete') {
        console.log(`[${green("phylum")}] All packages pass project thresholds.\n`)
    } else if (jobStatus.pass) {
        console.warn(`[${yellow("phylum")}] Unknown packages were submitted for analysis, please check again later.\n`);
        Deno.exit(126);
    } else {
        console.error(`[${red("phylum")}] The operation caused a threshold failure.\n`);
        Deno.exit(127);
    }
}

// Analyze new dependencies with phylum before install/update.
if (Deno.args.length >= 1
    && (
        Deno.args[0] === 'add')
        || Deno.args[0] === 'install'
        || Deno.args[0] === 'up'
        || Deno.args[0] === 'dedupe'
   ) {
    await checkDryRun(Deno.args[0], Deno.args.slice(1));
}

// Run the command with side effects.
console.log(`[${green("phylum")}] Applying changes…`);
let status = await Deno.run({ cmd: ['yarn', ...Deno.args] }).status();
Deno.exit(status.code);
