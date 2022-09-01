
import {PhylumApi} from "phylum";

type NullableString = string | null;

type PipVersionTuple = {
    major: number,
    minor: number,
    fix: number
};

type PackageEntry = {
  name: string,
  version: string
};

type PackageList = {
    package_type: string,
    packages: PackageEntry[]
}

type ParseState = {
    state: string,
    name: NullableString,
    version: NullableString,
};

type ParseStep = (a: string[], b: ParseState) => ParseState;
type NullableParseStep = ParseStep | null | undefined;

interface ParseMap {
    [name: string]: ParseStep
}

// Attempt to locate pip by running the provided command. Returns
// the provided path if pip can be located, or null if it can't.
async function tryRunPip(path: string): Promise<NullableString> {
    try {
        const res = await Deno.run({
            cmd: [path, '--version'],
            stdout: 'piped',
            stderr: 'piped',
        });

        await res.status();
        res.close();

        return path;
    } catch(e) {
        return null;
    }
}

// Tries to locate pip3, followed by pip.
// If the env var PHYLUM_PIP_PATH is present,
// then we will default to that first.
// This allows the pip version used to be controlled
// by the end user.
async function findPip(): Promise<NullableString> {
    const testPath = Deno.env.get('PHYLUM_PIP_PATH');
    if(testPath) {
        const path = await tryRunPip(testPath);
        if(path)
            return path;
    }
    const firstTry = await tryRunPip("pip3");
    if(!firstTry)
        return await tryRunPip("pip");

    return firstTry;
}

// Provides back the parsed version of the current pip installation.
async function getVersion(pipPath: string): Promise<PipVersionTuple> {
    const res = await Deno.run({
        cmd: [pipPath, '--version'],
        stdout: 'piped',
        stderr: 'piped'
    });

    try {
        const data = await res.output();

        const textResult = new TextDecoder().decode(data);

        const matchData = textResult.match(/pip\s+(\d+\.\d+\.\d+).*/);

        if (!matchData || matchData.length !== 2)
            throw new Error("unexpected output from pip");
        // This should be the pip version
        const versionData = matchData[1];
        const versionOutput = versionData.split('.');
        if(versionOutput.length < 3)
            throw new Error("unexpected version information");

        return {
            major: parseInt(versionOutput[0]),
            minor: parseInt(versionOutput[1]),
            fix: parseInt(versionOutput[2]),
        };
    } finally {
        res.close();
    }
}



const binaryFormats = new Set(['.tar.gz', '.zip', '.whl', '.pyd']);

// Extracts the package version from the target string, which is of the form:
// Flask-2.2.2-py3-none-any.whl
// This is made complicated due to the fact that package names may not match
// the provided packageName, and can contain -, _, ., etc. Versions may not
// adhere to pep8 standards also, so that must be accounted for here.
// This method will ultimately return the package/version tuple.
function parsePackageVersion(packageName: string, target: string): string[] {
    if(target.length < (packageName.length+2))
        throw new Error(`invalid target ${target} does not contain the package name ${packageName}`);

    // We will parse the real package name out of the target string
    // if it isn't the same as the package name provided. This happens with some packages -
    // consider flask shows up as Flask here. At present, our heuristic is that the beginning of the
    // next field (which is the package version) must start with the pattern `-[0-9]`; there are no
    // other guarantees, so that is what we start by looking for.
    let fixedName = packageName;
    if(!target.startsWith(packageName)) {
        const tmp: string[] = [];
        for(let i = 0; i < target.length; i++) {
            if(target[i] === '-' && i < (target.length - 1)) {
                const check = target[i+1];
                // If value is ascii '0'-'9'
                if(check.charCodeAt(0) <= 57 && check.charCodeAt(0) >= 48)
                    break;
            }

            tmp.push(target[i]);
        }

        fixedName = tmp.join('');
    }
    // Trim off the first portion containing the parsed name
    const rem = target.substring(fixedName.length+1);
    const [versionFirstPart, ...rest] = rem.split('-');

    // Now we will validate the version portion of the name.
    // Some versions contain weird artifacts - like binary extensions - at the end. We
    // will make a best effort at cleaning this up.
    const verList: string[] = [];
    for(let i = 0; i < versionFirstPart.length; i++) {
        if (versionFirstPart[i] === '.' && i < (versionFirstPart.length - 1)) {
            // Check to see if the next part of the value is non-numeric. This may not mean that we are
            // dealing with something we need to get rid of (as versions _could_ have rc or alpha versions or similar),
            // but we will now need to check.
            if (versionFirstPart[i + 1].charCodeAt(0) > 57 || versionFirstPart[i + 1].charCodeAt(0) < 48) {
                const nextValue = versionFirstPart.substring(i);
                // Now we need to check to see if this is a raw binary file or not - some "version" strings
                // are simply just a tarball or wheel file.
                if (binaryFormats.has(nextValue))
                    break;
            }
        }

        verList.push(versionFirstPart[i]);
    }

    const verFixed = verList.join('');

    if(rest.length) {
        const nextPart: string[] = [];

        for(const value of rest) {
            // we will skip the start of the target tuple - this means that we are now _really_ past the version info
            if (value.startsWith('py') || value.startsWith('cp'))
                break;

            nextPart.push(value);
        }
        // We want to make sure that we respect complex versions, which may contain a `-rcX` or similar.
        const versionFixed = nextPart.length ? `${verFixed}-${nextPart.join('-')}` : verFixed;

        return [fixedName, versionFixed];
    }

    return [fixedName, verFixed];
}

// This unpacks the list of packages collected by the pip dry run, and
// returns it as a list.
function parseOutput(output: string): PackageEntry[] {
    const res: PackageEntry[] = [];

    const unpacked = output.split('\n');
    if(unpacked.length < 3) {
        throw new Error(`package test failed - unexpected output from pip: ${output}`);
    }

    // The verbs here reflect the first word in each line we need to process.
    // We _should_ see a `Collecting <package name> <version constraints>`
    // message, followed by either `Using <package target string>` or
    // `Downloading <package target string>`.
    const actions: ParseMap = {
        'Collecting': (current: string[], _: ParseState): ParseState => {
            // Example input -> `Collecting itsdangerous>=2.0`; when we hit
            // this function call it _should_ be -> `["itsdangerous>=2.0"]`.
            // Note that this could include much more complex patterns, particularly
            // on the version constraint side, such as `asgiref<4,>=3.5.2`.
            if(current.length !== 1)
                throw new Error(`unexpected input for 'Collecting' from pip: ${current.join(' ')}`);

            const cValue = current[0];

            const res = cValue.match(/([A-Za-z0-9.\-_]+)(.*)/);
            if(!res || !res.length)
                throw new Error(`invalid collection line provided - ${cValue}`);

            const name: NullableString = res.length > 1 ? res[1] : res[0];
            const version: NullableString = res.length > 2 && res[2] ? res[2] : null;

            return {
                state: 'collection',
                name: name,
                version: version,
            };
        },
        'Using': (current: string[], last: ParseState): ParseState => {
            // Example input -> `Using cached Flask-2.2.2-py3-none-any.whl`; when
            // we hit this method, it should be `["cached", "Flask-2.2.2-py3-none-any.whl"]`
            if(last.state !== 'collection' || !last.name)
                throw new Error(`invalid parse state for output - ${last.state}; value: ${current.join(' ')}`);

            if(current.length <= 2 || current[0] !== 'cached')
                throw new Error(`invalid input provided for 'Using' clause: ${current.join(' ')}`);

            // Package detail in the example above would be: "Flask-2.2.2-py3-none-any.whl"
            const packageDetail = current[1];
            const [name, version] = parsePackageVersion(last.name, packageDetail);

            // Add to our current list of packages
            res.push({name: name, version: version});

            return {
                state: 'using',
                name: name,
                version: version,
            };
        },
        'Downloading': (current: string[], last: ParseState): ParseState => {
            // This is essentially the same as 'Using', minus the word "cached" being included.
            // Example:  `Downloading Django-4.1-py3-none-any.whl (8.1 MB)`
            // which would make `current` contain `["Django-4.1-py3-none-any.whl", "(8.1 MB)"]`.
            if(last.state !== 'collection' || !last.name)
                throw new Error(`invalid parse state for output - ${last.state}; value: ${current.join(' ')}`);

            const packageDetail = current[0];
            const [name, version] = parsePackageVersion(last.name, packageDetail);

            res.push({name: name, version: version});
            return {
                state: 'using',
                name: name,
                version: version,
            };
        },
        'Would': (current: string[], last: ParseState): ParseState => {
            // Not currently used, but included here for future use; this would receive a list
            // of _new_ packages only that would be installed; we are currently just capturing the total
            // list. Example input: `Would install Django-4.1 asgiref-3.5.2 backports.zoneinfo-0.2.1 sqlparse-0.4.2`
            // Which means current would contain:
            // `["install", "Django-4.1", "asgiref-3.5.2", "backports.zoneinfo-0.2.1", "sqlparse-0.4.2"]`
            return {
                state: 'end',
                name: null,
                version: null,
            };
        },
    };

    // This loop walks the lines of output from the pip dry run.
    // We process each line we have a defined action for (in the ParseMap above),
    // and skip ones that we don't. This could include console output with download
    // progress indicators, and other similar artifacts that we will essentially drop.
    let lastOutput: ParseState = {state: 'initial', name: null, version: null};
    for(let i = 0; i < unpacked.length; i++) {
        const trimmed = unpacked[i].trim();
        const current: string[] = trimmed.split(" ");
        if(!current.length)
            throw new Error("invalid input provided");

        const key = current[0];
        const action: NullableParseStep = actions[key];

        if(!action)
            continue;

        lastOutput = action(current.slice(1), lastOutput);
    }


    return res;
}

// We will essentially run pip with the provided arguments, parse the package output, and return the package list.
async function checkAndRunPip(args, pipPath: string): Promise<PackageList> {
    let packages: PackageEntry[] = [];

    const pipVersion = await getVersion(pipPath);
    if(pipVersion.major < 22 || pipVersion.minor < 2 || pipVersion.fix < 2) {
        throw new Error(
            `pip version ${pipVersion.major}.${pipVersion.minor}.${pipVersion.fix} found is too old, please upgrade to at least 22.2.2`);
    }

    const proc = Deno.run({
        cmd: [pipPath, 'install', ...args, '--dry-run', '-I'],
        stdout: 'piped',
        stderr: 'piped'
    });

    try {
        const rawOutput = await proc.output();
        const textOutput = new TextDecoder().decode(rawOutput);

        packages = parseOutput(textOutput);
    } finally {
        proc.close();
    }

    return {
        package_type: "pypi",
        packages: packages,
    }
}


const pipPath = await findPip();
if(!pipPath) {
    console.error("[phylum] Unable to find pip - please ensure it is available in the current environment, or provide.");
    console.error("[phylum] via the `PHYLUM_PIP_PATH` environment variable.");
    Deno.exit(1);
}


// Perform pip dry run install, and check the returned package list.
// If that succeeds, proceed with the pip install.
try {
    if(Deno.args[0] === 'install') {
        const packageList = await checkAndRunPip(Deno.args.slice(1), pipPath);
        const aid = await PhylumApi.analyze('pypi', packageList);
        const res = await PhylumApi.getJobStatus(aid);

        if(res.pass && res.status === 'complete') {
            console.log('[phylum] All packages pass project thresholds.\n');
        } else if (res.pass) {
            console.warn('[phylum] Unknown packages were submitted for analysis, please check against later.\n');
            Deno.exit(126);
        } else {
            console.error('[phylum] The operation caused a policy violation.\n');
            Deno.exit(127);
        }
    }
} catch(e) {
    console.error(`[phylum] Failed to run pip installation operation. Error: ${e}`);
    Deno.exit(-1);
}

const proc = await Deno.run({
    cmd: [pipPath, ...Deno.args],
});

const status = await proc.status();

proc.close();


Deno.exit(status.code);
