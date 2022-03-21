---
title: Analyzing Dependencies
category: 61e72e3a50a88e001a92ee5d
---

The Phylum CLI natively supports processing the lock/requirements files for several ecosystems, namely:
* NPM
* Ruby Gems
* PyPI

After setting up a Phylum [project](https://docs.phylum.io/docs/projects) , you can begin analysis by running:

```sh
phylum analyze <package-lock-file.ext>
```

The default response will provide you with a high-level overview of your packages, including the total project score, score distributions across all packages, whether or not this analysis was a pass or fail and the total number of packages still processing.

```
$ phylum analyze package-lock.json
✅ Job ID: 3cd30a5b-eeee-4ba1-b8e1-276c61e6502c


          Project: example-project                                         Label: uncategorized
       Proj Score: 5                                                        Date: 2021-08-19 19:04:55 UTC
         Num Deps: 70                                                     Job ID: 3cd30a5b-eeee-4ba1-b8e1-276c61e6502c
             Type: NPM                                                  Language: Javascript
          User ID: louis@phylum.io                             View in Phylum UI: https://app.phylum.io/projects/bce673f2-fd77-48cb-8ca6-80a51033a34d

     Score       Count
      0 - 10   [    1]                                                                                     Thresholds:
     10 - 20   [    2] █                                                                                Project Score: 0.6
     20 - 30   [    0]                                                                        Malicious Code Risk MAL:   0
     30 - 40   [    0]                                                                         Vulnerability Risk VLN:   0
     40 - 50   [    0]                                                                           Engineering Risk ENG:   0
     50 - 60   [    1]                                                                                Author Risk AUT:   0
     60 - 70   [    0]                                                                               License Risk LIC:   0
     70 - 80   [    1]
     80 - 90   [    3] █
     90 - 100  [   62] ████████████████████████████████

           Status: FAIL
           Reason: Project failed due to project score threshold of 0.6 not being met
```

You can get more detailed output from the analysis, to include specific issues and their severity, by using the `--verbose` flag:

```sh
phylum analyze --verbose <package-lock-file.ext>
```

If you prefer JSON formatted output, you can leverage the `--json` flag.

```sh
phylum analyze --verbose --json <package-lock-file.ext> > output.json
```
