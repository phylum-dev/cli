//
// Run this with `deno run --allow-all tests.ts`.
//
// The extension has the capability of running external commands (`poetry`) and
// change the working directory it is executed in.
//
// This file is not meant as a comprehensive integration test suite, but rather
// it is intended to provide an automated way of copying test fixtures to a
// temporary directory, run the extension there, and subsequently clean up.
//

import { describe, afterAll, beforeAll, it, } from "https://deno.land/std@0.148.0/testing/bdd.ts"
import { copy } from "https://deno.land/std@0.148.0/fs/mod.ts"
import { assert } from "https://deno.land/std@0.148.0/testing/asserts.ts"

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
      stdout: 'inherit',
      stderr: 'inherit',
    })


    const status = await process.status()
    await process.close()

    return status
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
  // The `poetry_test` project should be pre-existing.
  await phylum.run(['project', 'link', 'poetry_test'], phylum.fixturesPath)
})

afterAll(async () => {
  await phylum.cleanup()
})

describe("Poetry extension", async () => {
  it("correctly handles the `--dry-run` argument", async () => {
    // At this stage, we expect a return code of 1 since the packages aren't analyzed in staging.
    let status = await phylum.run(['poetry', 'add', '--dry-run', 'pandas'], phylum.fixturesPath)
    assert(status.code === 1)
  })

  it("correctly allows a valid package", async () => {
    // At this stage, we expect a return code of 1 since the packages aren't analyzed in staging.
    let status = await phylum.run(['poetry', 'add', 'pandas'], phylum.fixturesPath)
    assert(status.code === 1)
  })

  it("allows duplicating the `--lock` flag", async () => {
    // At this stage, we expect a return code of 1 since the packages aren't analyzed in staging.
    let status = await phylum.run(['poetry', 'add', '--lock', 'numpy'], phylum.fixturesPath)
    assert(status.code === 1)
  })

  it("correctly passes through other commands", async () => {
    let status

    status = await phylum.run(['poetry', 'check'], phylum.fixturesPath)
    assert(status.code === 0)

    status = await phylum.run(['poetry', 'help'], phylum.fixturesPath)
    assert(status.code === 0)

    status = await phylum.run(['poetry', 'version'], phylum.fixturesPath)
    assert(status.code === 0)
  })
})
