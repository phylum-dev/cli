#!/bin/sh

# usage: ./tag.sh [BRANCH]
# Tag the latest release commit (i.e., a commit with a "Release-Version" trailer)
#
# This script should be run on the default branch after a version bump (using bump_version.sh) has
# been merged to the default branch. It will find and tag the latest release commit. If no branch
# argument is provided, the search will begin from HEAD.
#
# Note: After the commit is found, a check is performed to ensure that the commit is reachable from
# the default branch on the remote (origin/HEAD). To skip this check, set SKIP_ORIGIN_HEAD_CHECK=1

set -eu

SOURCE_BRANCH=HEAD
if [ -n "${1:-}" ]; then
    SOURCE_BRANCH="$1"
fi

echo "Searching for latest release commit from ${SOURCE_BRANCH}"

TAG_COMMIT=$(git rev-list -i --grep='^release-version:' -1 "${SOURCE_BRANCH}")
if [ -z "${TAG_COMMIT}" ]; then
    echo "No release commit found!" >&2
    exit 1
fi

TAG=$(git log --pretty="format:%(trailers:key=release-version,valueonly)" -1 "${TAG_COMMIT}")
SUMMARY=$(git log --pretty="format:%(trailers:key=release-summary,valueonly)" -1 "${TAG_COMMIT}")

if git show "tags/${TAG}" > /dev/null 2>&1; then
    echo "Tag ${TAG} already exists!" >&2
    exit 1
fi

echo "Tagging this commit:"
git log --oneline -1 "${TAG_COMMIT}"

if [ -z "${SKIP_ORIGIN_HEAD_CHECK:-}" ]; then
    # Check that the tag is being created on the default branch
    git fetch origin >/dev/null
    if ! git merge-base --is-ancestor "${TAG_COMMIT}" origin/HEAD; then
        echo "WARNING! You are about to tag a commit that is not on the default branch!"
        echo "Unless you are patching an old version, this is probably not what you want!"

        printf "Are you sure? [y/N] "
        read -r yn
        if [ "${yn}" != "y" ] && [ "${yn}" != "Y" ]; then
            echo "Aborting tag"
            exit 1
        fi
    fi
fi

git tag --sign -m "${TAG} - ${SUMMARY}" "${TAG}" "${TAG_COMMIT}"

printf "\nOutput of the command: git show %s\n" "${TAG}"
git show "${TAG}"

cat << __instructions__

Successfully created tag!
Run the following command manually to push the new tag:

    git push origin ${TAG}

__instructions__
