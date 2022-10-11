import { PhylumApi } from 'phylum';

let cmd = PhylumApi.runSandboxed({
  cmd: '/bin/echo',
  args: ['hello'],
  stdout: 'piped',
  stderr: 'piped',
  exceptions: {
    read: false,
    write: false,
    net: false,
    run: false,
  }
});

await Deno.stdout.write(new TextEncoder().encode(cmd.stdout));
