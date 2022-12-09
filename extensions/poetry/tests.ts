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

import {
  afterAll,
  beforeAll,
  describe,
  it,
} from "https://deno.land/std@0.148.0/testing/bdd.ts";
import { copy } from "https://deno.land/std@0.148.0/fs/mod.ts";
import { assert } from "https://deno.land/std@0.148.0/testing/asserts.ts";

class Phylum {
  readonly tempDir: string;
  readonly fixturesDir: string;
  readonly extDir: string;

  constructor(tempDir: string, extDir: string) {
    this.tempDir = tempDir;
    this.fixturesDir = tempDir + "/fixtures";
    this.extDir = extDir;
  }

  async run(args: string[], cwd?: string) {
    let process = Deno.run({
      cmd: ["phylum", ...args],
      env: { XDG_DATA_HOME: await this.tempDir },
      cwd,
      stdout: "inherit",
      stderr: "inherit",
    });

    const status = await process.status();
    await process.close();

    return status;
  }

  async runExt(args: string[], cwd?: string) {
    return await this.run(
      ["extension", "run", "-y", this.extDir, ...args],
      cwd,
    );
  }

  async cleanup() {
    await Deno.remove(this.tempDir, { recursive: true });
  }
}

const phylum = new Phylum(await Deno.makeTempDir(), Deno.cwd());

beforeAll(async () => {
  await copy("./fixtures", phylum.fixturesDir);
  // The `poetry_test` project should be pre-existing.
  await phylum.run(["project", "link", "poetry_test"], phylum.fixturesDir);
});

afterAll(async () => {
  await phylum.cleanup();
});

describe("Poetry extension", async () => {
  // These tests may fail if the packages aren't processed on staging.

  it("correctly handles the `--dry-run` argument", async () => {
    let status = await phylum.runExt(
      ["add", "--dry-run", "numpy"],
      phylum.fixturesDir,
    );
    assert(status.code === 0);
  });

  it("correctly allows a valid package", async () => {
    let status = await phylum.runExt(["add", "numpy"], phylum.fixturesDir);
    assert(status.code === 0);
  });

  it("correctly denies a known bad package", async () => {
    let status = await phylum.runExt(
      ["add", "cffi==1.15.0"],
      phylum.fixturesDir,
    );
    assert(status.code !== 0);
  });

  it("allows duplicating the `--dry-run` flag", async () => {
    let status = await phylum.runExt(
      ["add", "--dry-run", "numpy"],
      phylum.fixturesDir,
    );
    assert(status.code === 0);
  });

  it("allows duplicating the `--lock` flag", async () => {
    let status = await phylum.runExt(
      ["add", "--lock", "numpy"],
      phylum.fixturesDir,
    );
    assert(status.code === 0);
  });

  it("correctly passes through other commands", async () => {
    let status;

    status = await phylum.runExt(["check"], phylum.fixturesDir);
    assert(status.code === 0);

    status = await phylum.runExt(["help"], phylum.fixturesDir);
    assert(status.code === 0);

    status = await phylum.runExt(["version"], phylum.fixturesDir);
    assert(status.code === 0);
  });
});
