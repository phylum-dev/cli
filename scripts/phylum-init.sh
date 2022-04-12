#!/bin/sh

set -eu

SUPPORTED_TARGETS="aarch64-apple-darwin x86_64-apple-darwin x86_64-unknown-linux-musl"

BASE_URL="https://github.com/phylum-dev/cli/releases/latest/download"

# Get the platform name.
get_platform() {
    platform=$(uname -s | tr '[:upper:]' '[:lower:]')

    case "${platform}" in
        linux) platform="unknown-linux-musl" ;;
        darwin) platform="apple-darwin" ;;
    esac

    echo "${platform}"
}

# Get the architecture
get_arch() {
    arch=$(uname -m | tr '[:upper:]' '[:lower:]')

    case "${arch}" in
        arm64) arch="aarch64" ;;
        amd64) arch="x86_64" ;;
    esac

    echo "${arch}"
}

# Get the target triple
get_target_triple() {
    echo "$(get_arch)-$(get_platform)"
}

# Check if a target is supported
is_supported() {
    target="$1"

    for tgt in ${SUPPORTED_TARGETS}; do
        if [ "${target}" = "${tgt}" ]; then
            return 0
        fi
    done

    return 1
}

# Check for a required command
require_command() {
    if ! type "$1" > /dev/null; then
        echo "This script requires \`$1\`. Please install it and re-run this script to continue." >&2
        exit 1
    fi
}

# All of the required commands (outside of POSIX)
require_command mktemp
require_command curl
require_command unzip

# Parse command line arguments
while [ "$#" -gt 0 ]; do
    case "$1" in
        -t | --target)
            TARGET=$2
            shift 2
            ;;
        -y | --yes)
            SKIP_CONFIRM=1
            shift 1
            ;;
        *)
            echo "Unsupported option: $1" >&2
            exit 1
            ;;
    esac
done

if [ -z "${TARGET:-}" ]; then
    TARGET="$(get_target_triple)"
fi

if ! is_supported "${TARGET}"; then
    echo "ERROR: Target not supported: ${TARGET}" >&2
    exit 1
fi

URL="${BASE_URL}/phylum-${TARGET}.zip"
echo "Release archive URL: ${URL}"

if [ -z "${SKIP_CONFIRM:-}" ]; then
    printf "Continue install? [y/N] "
    read -r yn
    if [ "${yn}" != "y" ] && [ "${yn}" != "yes" ]; then
        echo "Aborting install"
        exit 1
    fi
fi

tempdir="$(mktemp -d)"
archive="${tempdir}/phylum.zip"

# Download and extract
curl --fail --silent --show-error --location --output "${archive}" "${URL}"
unzip -qq "${archive}" -d "${tempdir}"

install_script="${tempdir}/phylum-${TARGET}/install.sh"
if ! [ -f "${install_script}" ]; then
    echo "ERROR: install.sh not found in the downloaded archive" >&2
    exit 1
fi

# Run the installer
sh "${install_script}"

# Cleanup the temporary directory
rm -r "${tempdir}"
