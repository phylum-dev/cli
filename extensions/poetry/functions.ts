export type PackageSpec = {
  name: string,
  version: string,
}

function isNotNull<T>(val: T | null): val is T {
  return val !== null
}

export function parseDryRunLine(line: string): PackageSpec | null {
  const installingRegexp = /Installing\s+([^\s]+)\s+\(([^\)]+)\)/

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

export function parseDryRun(output: string): PackageSpec[] {
  return output
    .split('\n')
    .map(parseDryRunLine)
    .filter(isNotNull)
}
