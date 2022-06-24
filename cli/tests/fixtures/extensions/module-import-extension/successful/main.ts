import { crypto } from "https://deno.land/std@0.143.0/crypto/mod.ts"
import { something } from "./local_module.ts"
import { notTypescript } from "./not_typescript.js"

let value = something()
console.log(`I should contain ${value}`)

