let response = await fetch('https://www.phylum.io')
let release = await response.text()

console.log(release)
