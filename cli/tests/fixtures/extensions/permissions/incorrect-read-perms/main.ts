import { walk, exists } from "https://deno.land/std@0.143.0/fs/mod.ts"

let data = new TextDecoder().decode(await Deno.readFile("/tmp/passwd"))
console.log(data)
