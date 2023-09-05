---
title: Analyzing Dependencies
category: 6255e67693d5200013b1fa41
hidden: false
---

After setting up a Phylum [project](https://docs.phylum.io/docs/create_project), you can begin analysis by running:

```sh
phylum analyze
```

The default response will provide an overall summary result to indicate whether the [project's established policy](https://docs.phylum.io/docs/policy) has been met. If there are still packages being processed, an incomplete status will be indicated. Any policy violations will be reported, along with a link to the complete report.

```shellsession
❯ phylum analyze
✅ Successfully parsed lockfile "/Users/demo/dev/phylum/example-project/./requirements.txt" as type: pip
✅ Successfully parsed lockfile "/Users/demo/dev/phylum/example-project/./package-lock.json" as type: npm
✅ Job ID: 3accba15-b0dc-43d2-b8ce-f5700360e3bd

Phylum Supply Chain Risk Analysis — FAILURE

[npm] cacheable-request@6.1.0
  [VLN] cacheable-request@6.1.0 is vulnerable to Regular Expression Denial of Service
[npm] ci-info@3.8.0
  [AUT] Author of ci-info@3.8.0 is using a disposable email domain
[npm] trim@0.0.1
  [VLN] trim@0.0.1 is vulnerable to Regular Expression Denial of Service
[pypi] crpytography@0.1
  [MAL] crpytography@0.1 may be a typosquatted package
  [MAL] crpytography@0.1 is vulnerable to a dependency confusion attack.
[pypi] cryptography@38.0.4
  [VLN] cryptography@38.0.4 is vulnerable to Vulnerable OpenSSL included
[pypi] ghostscript@0.7
  [LIC] Commercial license risk detected in ghostscript@0.7
[pypi] pyyaml@5.3.1
  [VLN] PyYAML@5.3.1 is vulnerable to Improper Input Validation

You can find the interactive report here:
  https://app.phylum.io/projects/e5eab4d2-d27d-42ac-bbad-f3ff5c588f54?label=uncategorized
```

If you prefer JSON formatted output, you can leverage the `--json` flag.

```sh
phylum analyze --json > output.json
```

If the analysis failed to meet the project's thresholds, the command's exit code will be set to `100`.
