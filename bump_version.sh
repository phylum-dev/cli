#!/bin/sh

# To release a new version of the CLI:
#
# * Run this script on a branch to bump the version
# * Submit a PR for the version bump and, after approval, merge it to the default branch
# * Run tag.sh from the default branch

set -eu

LATEST=$(git describe --tags --abbrev=0 --exclude="*-rc*")
printf "Latest release: %s\n\n" "${LATEST}"
printf "Git log since latest release:\n"
git log --oneline HEAD "^${LATEST}"
printf "\n"

printf "version (w/o a leading 'v'): "
read -r version
TAG=v${version}

today=$(date '+%Y-%m-%d')
printf "\nUpdating CHANGELOG, bumping version, running 'cargo check', and adding files for commit ...\n\n"
sed -i'.bak' "s/\(## Unreleased\)/\1\n\n## ${version} - ${today}/" CHANGELOG.md
rm -f CHANGELOG.md.bak

sed -E -i'.bak' "s/^version = \"([^\"]*)\"/version = \"${version}\"/" cli/Cargo.toml
rm -f cli/Cargo.toml.bak
cargo check
git add Cargo.lock
git add CHANGELOG.md
git add cli/Cargo.toml

printf "\nUpdating extension changelog...\n"
sed -i'.bak' "s/\(## Unreleased\)/\1\n\n## ${version} - ${today}/" extensions/CHANGELOG.md
rm extensions/CHANGELOG.md.bak
git add extensions/CHANGELOG.md

commit_message="Bump to ${TAG}"
printf "\nFiles to be added and committed with message: \"%s\"\n\n" "${commit_message}"
git status

printf "Press enter to proceed with the commit..."
read -r _

git commit -F - <<EOF
${commit_message}

Release-Version: ${TAG}
EOF

git log --pretty=fuller -1

cat << __instructions__

Version bump successful!

__instructions__
