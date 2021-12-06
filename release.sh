#!/bin/bash

printf "version: "
read version

printf "changelog: "
read changelog

sed -E -i "1 s#^#* $version - $changelog\n#" CHANGELOG
sed -E -i "s/^version = \"([^\"]*)\"/version = \"$version\"/" lib/Cargo.toml
sed -E -i "s/^version: \"([^\"]*)\"/version: \"$version\"/" lib/src/bin/.conf/cli.yaml
git add CHANGELOG
git add lib/Cargo.toml
git add lib/src/bin/.conf/cli.yaml
git commit -m "Bump version"

#sed -E -i "0,/^$/s/^version = \"([^\"]*)\"/version = \"$version\"/" bindings/python/Cargo.toml

TAG=v${version}

echo Tagging / pushing ${TAG}, press any key to proceed...
read
git push
git tag ${TAG}
git push --tags

