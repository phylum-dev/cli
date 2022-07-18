//
// Automated tests for the "poetry" extension.
//

import { describe, afterAll, beforeAll, it, } from "https://deno.land/std@0.148.0/testing/bdd.ts"
import { copy } from "https://deno.land/std@0.148.0/fs/mod.ts"

class Phylum {
  readonly xdgDataHome: string
  readonly fixturesPath: string

  constructor (tempDir: string) {
    this.xdgDataHome = tempDir
    this.fixturesPath = tempDir + '/fixtures'
  }

  async run (args: string[], cwd?: string) {
    let process = Deno.run({
      cmd: ['phylum', ...args],
      env: { 'XDG_DATA_HOME': await this.xdgDataHome },
      cwd,
    })

    await process.status()
    await process.close()
  }

  async installExtension (extension: string) {
    await this.run(['extension', 'install', '-y', extension])
  }

  async cleanup () {
    await Deno.remove(this.xdgDataHome, { recursive: true })
  }
}

const phylum = new Phylum(await Deno.makeTempDir())

beforeAll(async () => {
  await phylum.installExtension('./.')
  await copy('./fixtures',  phylum.fixturesPath)
  await phylum.run(['project', 'link', 'poetry_test'], phylum.fixturesPath)
})

afterAll(async () => {
  // await phylum.run(['project', 'delete', 'poetry_test'], phylum.fixturesPath)
  await phylum.cleanup()
})

describe("Poetry extension", async () => {
  it("correctly allows a valid package", async () => {
    await phylum.run(['poetry', 'add', 'pandas'], phylum.fixturesPath)
  })
})
