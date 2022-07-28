import { parse } from "https://deno.land/std@0.148.0/flags/mod.ts"

type PackageSpec = {
  name: string,
  version: string,
}

// Parse a package name and version from a line of a poetry dry run output.
// Returns `null` if no match was found.
function parseDryRunLine(line: string): PackageSpec | null {
  const installingRegexp = /: selecting\s+([^\s]+)\s+\(([^\)]+)\)/

  let matches = line.match(installingRegexp)
  if (matches != null) {
    return {
      name: matches[1],
      version: matches[2],
    }
  } else {
    return null
  }
} 

// Parse all packages from a poetry dry run output.
// Filter out the non-matching lines.
export function parseDryRun(output: string): PackageSpec[] {
  function isNotNull<T>(val: T | null): val is T {
    return val !== null
  }

  return output
    .split('\n')
    .map(parseDryRunLine)
    .filter(isNotNull)
}

