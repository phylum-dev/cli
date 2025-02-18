let response = await fetch('https://www.veracode.com')
let release = await response.text()

console.log(release)
