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

import * as Functions from './parse.ts'

class Phylum {
  readonly tempDir: string
  readonly fixturesDir: string
  readonly extDir: string

  constructor (tempDir: string, extDir: string) {
    this.tempDir = tempDir
    this.fixturesDir = tempDir + '/fixtures'
    this.extDir = extDir
  }

  async run (args: string[], cwd?: string) {
    let process = Deno.run({
      cmd: ['phylum', ...args],
      env: { 'XDG_DATA_HOME': await this.tempDir },
      cwd,
      stdout: 'inherit',
      stderr: 'inherit',
    })

    const status = await process.status()
    await process.close()

    return status
  }

  async runExt (args: string[], cwd?: string) {
    return await this.run(['extension', 'run', '-y', this.extDir, ...args], cwd)
  }

  async cleanup () {
    await Deno.remove(this.tempDir, { recursive: true })
  }
}

const phylum = new Phylum(await Deno.makeTempDir(), Deno.cwd())

beforeAll(async () => {
  await copy('./fixtures',  phylum.fixturesDir)
  // The `poetry_test` project should be pre-existing.
  await phylum.run(['project', 'link', 'poetry_test'], phylum.fixturesDir)
})

afterAll(async () => {
  await phylum.cleanup()
})

describe("Unit tests", async () => {
  it("correctly parses installed packages", async () => {
    const fixture = `
      Updating dependencies
      Resolving dependencies...
         1: fact: fixture is 0.1.0
         1: derived: fixture
         1: fact: fixture depends on pandas (^1.4.3)
         1: selecting fixture (0.1.0)
         1: derived: pandas (>=1.4.3,<2.0.0)
      PyPI: 1 packages found for pandas >=1.4.3,<2.0.0
         1: fact: pandas (1.4.3) depends on python-dateutil (>=2.8.1)
         1: fact: pandas (1.4.3) depends on pytz (>=2020.1)
         1: fact: pandas (1.4.3) depends on numpy (>=1.21.0)
         1: selecting pandas (1.4.3)
         1: derived: numpy (>=1.21.0)
         1: derived: pytz (>=2020.1)
         1: derived: python-dateutil (>=2.8.1)
      PyPI: No release information found for numpy-0.9.6, skipping
      PyPI: No release information found for numpy-1.4.0, skipping
      PyPI: 14 packages found for numpy >=1.21.0
      PyPI: 6 packages found for pytz >=2020.1
      PyPI: No release information found for python-dateutil-0.1, skipping
      PyPI: No release information found for python-dateutil-2.0, skipping
      PyPI: 2 packages found for python-dateutil >=2.8.1
         1: fact: python-dateutil (2.8.2) depends on six (>=1.5)
         1: selecting python-dateutil (2.8.2)
         1: derived: six (>=1.5)
      PyPI: 18 packages found for six >=1.5
         1: selecting pytz (2022.1)
         1: selecting six (1.16.0)
         1: selecting numpy (1.23.1)
         1: Version solving took 0.038 seconds.
         1: Tried 1 solutions.
    `

    const parsed = Functions.parseDryRun(fixture)

    assert(parsed.find(c => c.name === 'six' && c.version == '1.16.0'))
    assert(parsed.find(c => c.name === 'numpy' && c.version == '1.23.1'))
    assert(parsed.find(c => c.name === 'pytz' && c.version == '2022.1'))
    assert(parsed.find(c => c.name === 'pandas' && c.version == '1.4.3'))
  })
})

describe("Poetry extension", async () => {
  // These tests may fail if the packages aren't processed on staging.

  it("correctly handles the `--dry-run` argument", async () => {
    let status = await phylum.runExt(['add', '--dry-run', 'pandas'], phylum.fixturesDir)
    assert(status.code === 0)
  })

  it("correctly allows a valid package", async () => {
    let status = await phylum.runExt(['add', 'pandas'], phylum.fixturesDir)
    assert(status.code === 0)
  })

  // TODO tqdm is artificially marked as not passing; replace with an actual
  // known-bad package.
  it("correctly denies an invalid package", async () => {
    let status = await phylum.runExt(['add', 'tqdm'], phylum.fixturesDir)
    assert(status.code === 1)
  })

  it("allows duplicating the `--dry-run` flag", async () => {
    let status = await phylum.runExt(['add', '--dry-run', 'numpy'], phylum.fixturesDir)
    assert(status.code === 0)
  })

  it("allows duplicating the `--lock` flag", async () => {
    let status = await phylum.runExt(['add', '--lock', 'numpy'], phylum.fixturesDir)
    assert(status.code === 0)
  })

  it("correctly passes through other commands", async () => {
    let status

    status = await phylum.runExt(['check'], phylum.fixturesDir)
    assert(status.code === 0)

    status = await phylum.runExt(['help'], phylum.fixturesDir)
    assert(status.code === 0)

    status = await phylum.runExt(['version'], phylum.fixturesDir)
    assert(status.code === 0)
  })
})
