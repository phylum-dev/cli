import { PhylumApi } from "phylum";

/**
 *  Parses a string representing a package version tuple. The format of this
 *  string is `<name>:<version>`. Anything after the last `:` is treated as
 *  the version.
 */
function parsePackagTuple(s: string) {
    let parts = s.split(":");

    if(parts.length < 2) {
        console.error(`Invalid package string \`${s}\``);
        return;
    }

    let version = parts.pop();
    let name = parts.join(':');
    return { "name": name, "version": version };
}

/**
 *  Given a `gradle-dependencies.txt` file generated by `gradle`, attempts to
 *  parse dependencies from the file.
 */
function parseGradleFile(data: string) {
    const testRuntimeClasspath = parseOutTestRuntimeClasspathSection(data);

    // If the line starts with a + or a | take everything on that line from the
    // first alpha character to the first whitespace character.
    const dependencies: string[] = testRuntimeClasspath
    .filter(line => line.length > 0) //ignore empty lines
    .filter(line => line[0] == '+' || line[0] == '|') //the line starts like a dependency line
    .filter(line => line.indexOf("+--- project :") < 0) //ignore lines that reference sibling projects
    .map(line => {
        //Normalize the lines, no more nesting
        const normalized = line.match(/^[[+-\\|\s]*(.*)$/);
        //Should go from |         |    \--- commons-logging:commons-logging:1.0.3 -> 1.1.1 to just commons-logging:commons-logging:1.0.3 -> 1.1.1
        if (normalized) {
            return normalized[1]
        }
        else {
            return ""
        }
    })
    .filter(line => line.length > 0) //ignore empty lines (failed the regex for some reason)
    .map(line => {
        //At this point we have a few cases of what the line could look like
        //(A dependency, with possible omission indicator) org.apache.hadoop:hadoop-core:1.0.2 (*)
        //(A constrained dependency) xmlenc:xmlenc:{strictly 0.52} -> 0.52 (c)
        //(A version resolved depenendecy) commons-logging:commons-logging:1.1 -> 1.1.1
        let parsed = line.match(/^([\.\w\-]+):([\.\w\-]+):{?(?:strictly)?\s?([\\.\w\-]+)}?\s?-?>?\s?([\{}\.\w\-]*)\s?[\w\*\(\)]*\s?$/);
        if (parsed) {
            const groupId = parsed[1];
            const artifactId = parsed[2];
            const originalVersion = parsed[3];
            const possibleReplacementVersion = parsed[4];

            const name = groupId + ":" + artifactId;
            const version = possibleReplacementVersion ? possibleReplacementVersion : originalVersion;
            return name + ":" + version
        }
        return "";
    })
    .filter(line => line.length > 0) //ignore empty lines (failed the regex for some reason);
    return [...new Set(dependencies)].map((x) => parsePackagTuple(x));
}

function parseOutTestRuntimeClasspathSection(gradleDependencies: string) {    
    const lines = gradleDependencies.split(/\r?\n/);
    //testRuntimeClasspath represents all dependencies including testing one    
    const sectionStart = lines.findIndex(x => x.startsWith("testRuntimeClasspath"));

    if (sectionStart < 0) {
        return [];
    }

    const beginingOfTestRuntimeClasspath = lines.slice(sectionStart, lines.length)

    //Find the end of the section
    const sectionEnd = beginingOfTestRuntimeClasspath.findIndex(x => x.length == 0);
    if (sectionEnd < 0) {
        return [];
    }
    
    return beginingOfTestRuntimeClasspath.slice(0, sectionEnd + 1);
}

/**
 *  Runs the `gradle` binary to produce the `gradle-dependencies.txt` file. This
 *  should do the dependency resolution for this machine.
 *
 *  We perform a rudimentary check to make sure there wasn't an outright build
 *  failure.
 */
async function generateGradleDeps() {
    let gradleResp = null;

    try {
        gradleResp = await Deno.run({
            cmd: ["gradle", "-q", "dependencies"],
            stdout: "piped"
        }).output();
    } catch(e) {
        console.error("It doesn't look like you have `gradle` installed");
        return;
    }

    // Conver the Uint8 array to ascii
    const ret = new TextDecoder().decode(gradleResp);

    if(ret.indexOf("BUILD FAILED") > 0) {
        return;
    }

    return ret;
}

/**
 *  Parse a `build.gradle` file and returned the identified dependencies. 
 */
async function getBuildGradleDeps(path: string, project: string, group: string) {
    console.log("[*] Parsing dependencies from `build.gradle`");
    console.warn("[!] WARNING: You should consider locking your dependencies and " +
                 "using `phylum analyze` instead.");
    console.warn("");
    console.warn("    See: https://docs.gradle.org/current/userguide/dependency_locking.html");

    const gradleDeps = await generateGradleDeps();

    if(!gradleDeps) {
        console.error("[!] ERROR: Failed to parse dependencies. Check your " +
                      "`build.gradle` file."); 
        return;
    }

    // Parse the dependencies from this file.
    let foundDeps = parseGradleFile(gradleDeps); 

    console.log(`\n[*] Found ${foundDeps.length} dependencies in \`build.gradle\``);
    console.log(foundDeps);
    console.log("");

    return foundDeps;
}

/**
 *  Submit the provided dependencies to Phylum for analysis.
 */
async function submit(pkgs: object[], project: string, group: string) {
    console.log("[*] Submitting to Phylum for analysis...");

    // If we don't have a specified group/project from the command line arguments,
    // attempt to parse from our Phylum project file.
    if(!group && !project) {
        console.log("[*] Using details from `.phylum_project` file");
        const projectFile = await PhylumApi.getProjectDetails();
        project = projectFile.name;
        group = projectFile.group;
    }

    if(group && !project) {
        console.error("[!] ERROR: You cannot specify a group without a project.");
        return;
    }

    if(project) {
        console.log(`\t --> Project: ${project}`)
    }

    if(group) {
        console.log(`\t --> Group: ${group}`)
    }

    if(!group && !project) {
        console.error("[!] ERROR: You must specify a project (and optionally a group).");
        return;
    }

    if(pkgs.length) {
        const jobId = await PhylumApi.analyze("maven", pkgs, project, group);
        console.log(`[*] Job submitted for analysis with job ID:\n\n\t${jobId}`);
    } else {
        console.info("[*] No packages to submit");
    }
}

/**
 *  A cheap CLI argument parser for our extension.
 */
function parseArg(arg: string, cliArgs: string[]) {
    for(let i = 0; i < cliArgs.length; i++) {
        if(cliArgs[i] === `--${arg}`) {
            if(i+1 < cliArgs.length) {
                return cliArgs[i+1];
            }

            break;
        }
    }

    return null;
}

// Parse CLI args.
const args = Deno.args;

let group = parseArg("group", args);
let project = parseArg("project", args);

let gradleDependencies = await getBuildGradleDeps();

if(gradleDependencies) {
    submit(gradleDependencies, project, group);
}
