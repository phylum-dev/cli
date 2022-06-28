let response = await fetch('https://api.github.com/repos/phylum-dev/cli/releases/latest')
let release = await response.json()

console.log(release)
