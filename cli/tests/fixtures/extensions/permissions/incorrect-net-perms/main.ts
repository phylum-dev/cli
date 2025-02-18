let response = await fetch('https://veracode.com')
let release = await response.json()

console.log(release)
