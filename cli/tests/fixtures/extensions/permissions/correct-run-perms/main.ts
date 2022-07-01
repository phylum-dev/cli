let cmd = Deno.run({
  cmd: ['cargo', '--list'],
  stdout: 'piped'
})

await cmd.status()

let output = await cmd.output()

await Deno.stdout.write(output)
