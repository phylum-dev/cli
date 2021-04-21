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
     'l': {
         alias: 'label',
         describe: 'A label (meaningful identifier) for this submission',
         type: 'string',
         required: true,
         nargs: 1,
      },
     't': {
         alias: 'type',
         describe: 'Type of file to process (`package` or `yarn`, defaults to `package`)',
         type: 'string',
         nargs: 1,
      },
      'd': {
         alias: 'dry-run',
         describe: 'Print the list of packages that would be submitted, but do not actually submit them',
         type: 'boolean',
      }
   })
   .help()
   .alias('help', 'h')
   .argv;

function parsePackageLock(file) {
   const package_info = JSON.parse(file)
   const dependencies = package_info['dependencies'];

   let recs = [];

   for (const key in dependencies) {
      if (!!max_packages && recs.length >= max_packages) {
         break;
      }

      recs.push({
         'name': key,
         'version': dependencies[key]['version']
      });

      if (debug) {
         console.log(`${key}:${dependencies[key]['version']}`);
      }
   }
   console.log(`Found ${recs.length} packages.`)
   return recs;
}

function parseYarnLock(file) {
   const json = lockfile.parse(file);
   const dependencies = json['object']

   let recs = [];

   for (const key in dependencies) {
      if (!!max_packages && recs.length >= max_packages) {
         break;
      }

      const dep_name = key.substring(0, key.lastIndexOf('@')); 
      recs.push({
         'name': dep_name,
         'version': dependencies[key]['version']
      });

      if (debug) {
         console.log(`${dep_name}:${dependencies[key]['version']}`)
      }
   }
   console.log(`Found ${recs.length} packages.`)
   return recs;
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

let packages = [];
if (is_yarn) {
   try {
      packages = parseYarnLock(file)
   } catch(e) {
      console.log("Invalid yarn file: " + e);
      usage();
   }
} else {
   try {
      packages = parsePackageLock(file)
   } catch(e) {
      console.log("Invalid package file: " + e);
      usage();
   }
}

if (packages.length == 0){
   console.log("No valid packages found.");
   usage();
}

if (argv['dry-run']) {
   exit(0);
}

// Submit the list of packages for ingestion and processing
let stdinStream = new stream.Readable();
for (const p of packages) {
  let cli_input = `${p['name']}:${p['version']}\n`
  stdinStream.push(cli_input);
}

stdinStream.push(null);

child = execFile('phylum', ['batch', '-t', 'npm', '-l', argv['label']], (err, stdout, stderr) => {
     console.log(stdout);
     console.log(stderr);
  });
stdinStream.pipe(child.stdin);

if (!!child.err) {
  console.log(child.err);
}
