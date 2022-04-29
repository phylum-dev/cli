#!/bin/sh

# Releasing a new version of the CLI is initiated with a tag and completed with the
# Release workflow in CI. Run this script from the `main` branch and follow the prompts
# to initiate a new release. There is a manual step required at the end. This is to
# ensure a chance to review the automated work and not accidentally release a new version.

set -eu

LATEST=$(git describe --tags --abbrev=0 --exclude="*-rc*")
printf "Latest release: %s\n\n" "${LATEST}"
printf "Git log since latest release:\n"
git log --oneline HEAD "^${LATEST}"
printf "\n"

printf "version (w/o a leading 'v'): "
read -r version
TAG=v${version}

printf "changelog (one line summary): "
read -r changelog

printf "\nUpdating CHANGELOG, bumping version, running 'cargo check', and adding files for commit ...\n\n"
sed -E -i'.bak' "1 s#^#* ${version} - ${changelog}\n#" CHANGELOG
rm -f CHANGELOG.bak
sed -E -i'.bak' "s/^version = \"([^\"]*)\"/version = \"${version}\"/" cli/Cargo.toml
rm -f cli/Cargo.toml.bak
cargo check
git add Cargo.lock
git add CHANGELOG
git add cli/Cargo.toml

commit_message="Bump to ${TAG} - ${changelog}"
printf "\nFiles to be added and committed with message: \"%s\"\n\n" "${commit_message}"
git status

printf "Press enter to proceed with the commit and tag ..."
read -r
git commit -m "${commit_message}"
git tag --sign -m "${TAG} - ${changelog}" "${TAG}"

printf "\nOutput of the command: git show %s\n" "${TAG}"
git show "${TAG}"

cat << __instructions__

The automation is done.
Run the following command manually to push the changes:

    git push origin main ${TAG}

__instructions__
