# Phylum pip-lock extension

A [Phylum CLI][phylum-cli] extension that takes a loose `requirements.txt` file
and generates strict output.

[phylum-cli]: https://github.com/phylum-dev/cli


## Installation and basic usage

Clone the repository and install the extension via the Phylum CLI.

```console
git clone https://github.com/phylum-dev/cli
phylum extension install cli/extensions/pip-lock
```

Generate strict requirements output by pointing the extension at your `requirements.txt`:

```console
phylum pip-lock ./requirements.txt
```


Have the extension submit the resultant strict requirements for analysis:

Note: use of the `--analyze` option requires that a `.phylum_project` file is present, and returns JSON output of 
the analysis job.

```console
phylum pip-lock ./requirements.txt --analyze
```
