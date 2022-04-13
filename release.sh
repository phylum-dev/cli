#!/bin/sh

LATEST=$(git describe --tags --abbrev=0)

printf "Latest release: %s\n\nGit log:\n" "${LATEST}"
git log --oneline HEAD "^${LATEST}"
printf "\n"

printf "version: "
read -r version

printf "changelog: "
read -r changelog

sed -E -i "1 s#^#* ${version} - ${changelog}\n#" CHANGELOG
sed -E -i "s/^version = \"([^\"]*)\"/version = \"${version}\"/" cli/Cargo.toml
git add CHANGELOG
git add cli/Cargo.toml
git commit -m "v${version} - ${changelog}"

TAG=v${version}

echo Tagging / pushing "${TAG}", press any key to proceed...
read -r
git push
git tag --sign -m "${TAG} - ${changelog}" "${TAG}"
git push "${TAG}"
