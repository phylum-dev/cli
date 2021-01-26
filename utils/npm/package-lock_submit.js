#!/usr/bin/env node
const fs = require('fs');
const lockfile = require('@yarnpkg/lockfile');
const yargs = require('yargs');
const { execFile, execFileSync } = require('child_process')
const stream = require('stream');
const { exit } = require('process');

const max_packages = 0; // max number of packages to submit 0 - no maximum
const debug = true;

let is_yarn = false;

const argv = yargs
   .options({
     't': {
         alias: 'type',
         describe: 'the type of the file to process (`package` or `yarn`, defaults to `package`)',
         type: 'string',
         nargs: 1,
      },
      'd': {
         alias: 'dry-run',
         describe: 'print the list of packages that would be submitted, but do not actually submit them',
         type: 'boolean',
      }
   })
   .help()
   .alias('help', 'h')
   .argv;

function parsePackageLock(file) {
   const package_info = JSON.parse(file)
   const dependencies = package_info['dependencies'];

   let buf = "";
   let pkgCount = 0;

   for (const key in dependencies) {
      //console.log(`${key}:${dependencies[key]['version']}`);
      if (!!max_packages && pkgCount > max_packages) {
         break;
      }
      pkgCount++;
      buf += `${key}:${dependencies[key]['version']}\n`;
   }
   console.log(`Found ${pkgCount} packages.`)
   if (debug) {
      console.log();
      console.log(buf);
   }
   if (pkgCount == 0) {
      return null;
   }
   return buf
}

function parseYarnLock(file) {
   const json = lockfile.parse(file);
   const dependencies = json['object']

   let buf = "";
   let pkgCount = 0;

   for (const key in dependencies) {
      if (!!max_packages && pkgCount > max_packages) {
         break;
      }
      pkgCount++;
      const dep_name = key.substring(0, key.lastIndexOf('@')); 
      //console.log(`${dep_name}:${dependencies[key]['version']}`)
      buf += `${dep_name}:${dependencies[key]['version']}\n`;
   }
   console.log(`Found ${pkgCount} packages.`)
   if (debug) {
      console.log();
      console.log(buf);
   }
   if (pkgCount == 0) {
      return null;
   }
   return buf
}

function usage() {
   yargs.showHelp();
   exit(-1);
}

if (argv.length <= 3) {
   usage();
}

if (argv['type'] == 'yarn') {
   is_yarn = true;
}

let inputfile = argv._[0];

let file;
try {
   file = fs.readFileSync(inputfile, 'utf8');
} catch(e) {
   console.log("Failed to open file: " + e);
   usage();
}

let cli_input;
if (is_yarn) {
   try {
      cli_input = parseYarnLock(file)
   } catch(e) {
      console.log("Invalid yarn file: " + e);
      usage();
   }
} else {
   try {
      cli_input = parsePackageLock(file)
   } catch(e) {
      console.log("Invalid package file: " + e);
      usage();
   }
}

if (cli_input == null){
   console.log("No valid packages found.");
   usage();
}

if (argv['dry-run']) {
   exit(0);
}

let stdinStream = new stream.Readable();
stdinStream.push(cli_input);
stdinStream.push(null);

child = execFile('phylum-cli', ['batch', '-t', 'npm'], (err, stdout, stderr) => {
      console.log(stdout);
      console.log(stderr);
   });
stdinStream.pipe(child.stdin);

if (!!child.err) {
    console.log(child.err);
}
