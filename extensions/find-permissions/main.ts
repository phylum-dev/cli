import { red, green, blue, yellow } from 'https://deno.land/std@0.150.0/fmt/colors.ts';
import { PhylumApi } from 'phylum';

// Print help.
if (Deno.args.includes('-h')
    || Deno.args.includes('help')
    || Deno.args.includes('--help'))
{
    console.log('find-permissions');
    console.log('CLI extension to help find required sandboxing permissions.');
    console.log();
    console.log('USAGE:');
    console.log('    phylum find-permissions [OPTIONS] --bin <PATH>');
    console.log();
    console.log('OPTIONS:');
    console.log('    --read               Check if tested paths need to be readable or executable');
    console.log('    --write              Check if tested paths need to be writable');
    console.log('    --pre-bin <PATH>     Executable to be run before test execution');
    console.log('    --bin <PATH>         Executable to be run to test path necessity');
    console.log('    --post-bin <PATH>    Executable to be run after test execution');
    console.log('    --skip-files         Only check directories, speeding up the process');
    console.log('    --strict             Use strict sandboxing mode');
    Deno.exit(0);
}

// Permissions types to be checked.
let check_write = Deno.args.includes('--write');
let check_read = Deno.args.includes('--read');

// Ensure at least one type of permission is specified.
if (!check_write && !check_read) {
    console.error('Expected at least one of `--read`, `--write`');
    Deno.exit(111);
}

// Ensure test executable was passed.
const test_bin_path = getArgOption('--bin');
if (!test_bin_path) {
    console.error('Missing required `--bin <PATH>`');
    Deno.exit(222);
}

// Get absolute test executable path.
let test_bin;
try {
    test_bin = await Deno.realPath(test_bin_path);
} catch (e) {
    console.error(`Invalid executable path: ${test_bin_path}`);
    Deno.exit(333);
}

// Get setup/teardown executables.
const pre_test_bin = getArgOption('--pre-bin');
const post_test_bin = getArgOption('--post-bin');

// Check if files should be ignored.
const skipFiles = Deno.args.includes('--skip-files');

// Check if sandboxing should be strict.
const strict = Deno.args.includes('--strict');

// Required sandboxing exceptions.
const requiredPaths = [];

// Run analysis and report results.
await checkPath([], '/');
console.log('\nRequired paths: [');
for (const path of requiredPaths) {
    console.log(`    ${yellow(`"${path}"`)},`);
}
console.log(']');

// Recursively check the path for required sandboxing exceptions.
async function checkPath(allowed: [string], path: string) {
    console.log(`${blue(`Scanning "${path}"...`)}`);

    // Return immediately if it works without the path.
    if (await test(allowed)) {
        console.log(`${green(`${path}: Unnecessary directory`)}`);
        return;
    }

    // Ensure path has trailing slash.
    if (!path.endsWith('/')) {
        path += '/';
    }

    // Get all files and directories in this folder.
    const directories = [];
    const files = [];
    for await (const entry of Deno.readDir(path)) {
        if (entry.isDirectory) {
            directories.push(path + entry.name);
        } else if (entry.isFile) {
            files.push(path + entry.name);
        }
    }

    // Add path if it doesn't work with all directories and files.
    const allowedAndChildren = allowed.concat(files).concat(directories);
    if (!(await test(allowedAndChildren))) {
        console.log(`${red(`${path}: Required directory`)}`);
        requiredPaths.push(path);
        return;
    }

    // Check if any file is required.
    const allowedAndDirectories = allowed.concat(directories);
    if (!(await test(allowedAndDirectories))) {
        if (skipFiles) {
            // Add entire directory if any file is required and we're skipping file checks.
            console.log(`${red(`${path}: Required directory due to skipping files`)}`);
            requiredPaths.push(path);
            return;
        } else {
            // Add all required files.
            for (const file of files) {
                const withoutFile = allowedAndChildren.filter(entry => entry != file);
                if (!(await test(withoutFile))) {
                    console.log(`${red(`${file}: Required file`)}`);
                    requiredPaths.push(file);
                } else {
                    console.log(`${green(`${file}: Unnecessary file`)}`);
                }
            }
        }
    }

    // Check if any directory is required.
    const allowedAndFiles = allowed.concat(files);
    if (!(await test(allowedAndFiles))) {
        // Check all child directories.
        for (const directory of directories) {
            const withoutDirectory = allowedAndChildren.filter(entry => entry != directory);
            await checkPath(withoutDirectory, directory);
        }
    }
}

// Check if execution with the specified directories works.
async function test(directories: [string]): bool {
    // Use directories for enabled permission types, allow everything otherwise.
    let write = ['/'];
    let read = ['/'];
    if (check_write) {
        write = directories;
    }
    if (check_read) {
        read = directories;
    }

    // Run pre-test setup executable.
    if (pre_test_bin) {
        let pre_status = PhylumApi.runSandboxed({
            cmd: pre_test_bin,
            stdout: 'null',
            stderr: 'null',
            exceptions: {
                write: true,
                read: true,
                execute: true,
                net: true
            }
        })

        if (!pre_status.success) {
            console.error(`${red('Pre-test executable failed')}`);

            // Assume test would fail if setup didn't even run.
            return false;
        }
    }

    // Add `test_bin` path to run permissions.
    read.push(test_bin);

    // Run test against test executable.
    let output = undefined;
    try {
        output = PhylumApi.runSandboxed({
            cmd: test_bin,
            exceptions: {
                strict,
                write,
                read,
                run: read,
                net: true,
            },
            stdout: 'null',
            stderr: 'null',
        });
    } catch (_e) {
        return false;
    }

    // Run post-test cleanup executable.
    if (post_test_bin) {
        let post_status = PhylumApi.runSandboxed({
            cmd: post_test_bin,
            stdout: 'null',
            stderr: 'null',
            exceptions: {
                write: ['/'],
                read: ['/'],
                execute: ['/'],
                net: true
            }
        })

        if (!post_status.success) {
            console.error(`${red('Post-test executable failed')}`);

            // Mark test as failed to exit as quickly as possible.
            return false;
        }
    }

    return output.success;
}

// Get the value of a CLI argument.
function getArgOption(option: string): string | undefined {
    let option_index = Deno.args.findIndex(arg => arg === option);
    if (option_index !== -1) {
        return Deno.args[option_index + 1];
    } else {
        return undefined;
    }
}
