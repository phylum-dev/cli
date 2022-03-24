#!/bin/bash

printf "version: "
read -r version

printf "changelog: "
read -r changelog

sed -E -i "1 s#^#* ${version} - ${changelog}\n#" CHANGELOG
sed -E -i "s/^version = \"([^\"]*)\"/version = \"${version}\"/" cli/Cargo.toml
git add CHANGELOG
git add cli/Cargo.toml
git commit -m "Bump version"

TAG=v${version}

echo Tagging / pushing "${TAG}", press any key to proceed...
read -r
git push
git tag "${TAG}"
git push --tags
