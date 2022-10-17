---
title: Extension Sandboxing
category: 62c5cb137dbdad00536291a6
hidden: false
---

Phylum's CLI extensions allow developers to impose additional restrictions when
running third party applications. This can protect the system from damage when
these applications contain vulnerabilities or execute untrusted code.

## Example

The following code provides an example on how you could sandbox `cat` to only
allow access to files in the current working directory or below it:

```ts
import { PhylumApi } from 'phylum';

// Ensure a file path is passed as the only argument.
if (Deno.args.length !== 1) {
    console.log("USAGE: local-cat <FILE>");
    Deno.exit(123);
}

// Run `cat` in our sandboxed environment.
const output = PhylumApi.runSandboxed({
    cmd: 'cat',
    args: [Deno.args[0]],
    stdout: 'inherit',
    stderr: 'inherit',
    exceptions: {
        read: ['./'],
        write: false,
        run: false,
        net: false,
    },
});

// Propagate `cat`'s exit code.
Deno.exit(output.code);
```

When running this against a file in your local directory, it will print its
content, otherwise, you'll see `cat` printing the following error:

```text
cat: /tmp/illegal: Permission denied
```

The important part in this code snippet is the `exceptions` field. By default
access to most system resources is restricted, so if you want to access them
from within the sandbox you'll have to add an exception.

Available fields for exceptions are `read`, `write`, `run`, and `net`. The `run`
permission is a superset of `read` that allows for execution. While `read`,
`write`, and `run` accept either a path to be allowed or a boolean, `net` only
allows for a boolean value.

## Limitations

The `PhylumApi.runSandboxed` method is the only allowed means of spawning
child processes from an extension. The `Deno.run` method is explicitly disabled
in order to prevent extension from escaping the sandbox.

The method is only allowed to request permissions that are at least as restrictive
as the ones specified in the manifest.

## Advanced Usage

By default, access to some paths is granted automatically to make extension
sandboxing easier and ensure portability between operating systems. If you want
more control over your sandboxing exceptions, you can pass `strict: true` to the
`exceptions`:

```ts
const output = PhylumApi.runSandboxed({
    cmd: 'cat',
    args: [Deno.args[0]],
    stdout: 'inherit',
    stderr: 'inherit',
    exceptions: {
        // Discard all default sandboxing exceptions.
        strict: true,

        // You can see the required additional exceptions here.
        // These paths might differ between operating systems and distributions.
        run: ['/usr/bin/cat', '/usr/lib'],

        // These are identical to our previous example.
        read: ['./'],
        write: false,
        net: false,
    },
});
```

## Finding Required Exceptions

It can be somewhat difficult to find out which exceptions you need to add to
allow your application to run without any errors. To simplify this a bit you can
use the [`find-permissions`] extension.

[`find-permissions`]: https://github.com/phylum-dev/cli/tree/main/extensions/find-permissions

This extension will run a script against each path in your filesystem
recursively to validate what the most granular necessary exceptions are. Once
completed, it will output all necessary paths.

Most invocations of this extension will probably look something like this:

```sh
phylum find-permissions \
    --read \
    --write \
    --pre-bin ./setup.sh \
    --bin ./test.sh \
    --post-bin ./cleanup.sh
```

By passing both `--read` and `--write` we check for both permissions at the same
time. The `test.sh` script should contain the executable we want to sandbox; in
our example it would run `cat` against some local files. The `--pre-bin` and
`--post-bin` are optional, but here we could setup and remove local files to run
`cat` against for example.

Since this crawls your entire directory tree, it might take some time. If you
don't need file-level granularity you can help speed it up by passing
`--skip-files`.
