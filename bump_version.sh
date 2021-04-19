#!/bin/bash

printf "version: "
read version

printf "changelog: "
read changelog

sed -E -i "1 s/^/* $version - $changelog\n/" CHANGELOG
sed -E -i "s/^version = \"([^\"]*)\"/version = \"$version\"/" lib/Cargo.toml
sed -E -i "s/^version: \"([^\"]*)\"/version: \"$version\"/" lib/src/bin/.conf/cli.yaml

sed -E -i "0,/^$/s/^version = \"([^\"]*)\"/version = \"$version\"/" bindings/python/Cargo.toml
