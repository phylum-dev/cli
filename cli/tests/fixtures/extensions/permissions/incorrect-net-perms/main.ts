let response = await fetch('https://phylum.io')
let release = await response.json()

console.log(release)
