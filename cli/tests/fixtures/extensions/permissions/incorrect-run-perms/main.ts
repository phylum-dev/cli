import { PhylumApi } from 'phylum';

let cmd = PhylumApi.runSandboxed({
  cmd: 'echo',
  args: ['hello'],
  stdout: 'piped',
  stderr: 'piped',
});

await Deno.stdout.write(new TextEncoder().encode(cmd.stdout));
