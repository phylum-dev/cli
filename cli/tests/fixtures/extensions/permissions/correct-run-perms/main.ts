let cmd = Phylum.runSandboxed({
  cmd: 'echo',
  args: ['hello'],
  stdout: 'piped',
  stderr: 'piped',
  exceptions: {
      run: ['echo'],
  },
});

await Deno.stdout.write(new TextEncoder().encode(cmd.stdout));
