#!/bin/sh

set -eu

SUPPORTED_TARGETS="aarch64-apple-darwin x86_64-apple-darwin x86_64-unknown-linux-musl"
BASE_URL="https://github.com/phylum-dev/cli/releases/latest/download"
MINISIG_PUBKEY="RWT6G44ykbS8GABiLXrJrYsap7FCY77m/Jyi0fgsr/Fsy3oLwU4l0IDf"

# Get the platform name.
get_platform() {
    platform=$(uname -s | tr '[:upper:]' '[:lower:]')

    case "${platform}" in
        linux) platform="unknown-linux-musl" ;;
        darwin) platform="apple-darwin" ;;
        *) ;;
    esac

    echo "${platform}"
}

# Get the architecture
get_arch() {
    arch=$(uname -m | tr '[:upper:]' '[:lower:]')

    case "${arch}" in
        arm64) arch="aarch64" ;;
        amd64) arch="x86_64" ;;
        *) ;;
    esac

    echo "${arch}"
}

# Get the target triple
get_target_triple() {
    arch="$(get_arch)"
    platform="$(get_platform)"
    echo "${arch}-${platform}"
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

# Download the file to the given location
download() {
    curl --fail --silent --show-error --location --output "$1" "$2"
}

# Check for a required command
require_command() {
    cmd="$1"
    help_msg="${2:-}"

    if ! type "${cmd}" > /dev/null 2>&1; then
        echo "ERROR: This script requires \`${cmd}\`. Please install it and re-run this script to continue." >&2
        if [ -n "${help_msg}" ]; then
            printf "\n" >&2
            echo "${help_msg}" >&2
        fi
        exit 1
    fi
}

usage() {
    cat 1>&2 <<EOF
phylum-init.sh [options]

Fetch and install the phylum CLI.

Options
    -t, --target       Specify the target triple to install
    -y, --yes          Do not prompt for confirmation during install
    --no-verify        Disable signature verification (NOT RECOMMENDED)
    -h, --help         Show this help message
EOF
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
        --no-verify)
            NO_VERIFY=1
            shift 1
            ;;
        -h | --help)
            usage
            exit 0
            ;;
        *)
            printf "Unsupported option: %s\n\n" "$1" >&2
            usage
            exit 1
            ;;
    esac
done

if [ -z "${NO_VERIFY:-}" ]; then
    require_command minisign "See https://jedisct1.github.io/minisign/ for information on installing minisign"
fi

if [ -z "${TARGET:-}" ]; then
    TARGET="$(get_target_triple)"
fi

# shellcheck disable=SC2310
if ! is_supported "${TARGET}"; then
    echo "ERROR: Target not supported: ${TARGET}" >&2
    exit 1
fi

URL="${BASE_URL}/phylum-${TARGET}.zip"
echo "Release archive URL: ${URL}"

if [ -z "${SKIP_CONFIRM:-}" ]; then
    printf "Continue install? [y/N] "
    read -r yn
    yn="$(echo "${yn}" | tr "[:upper:]" "[:lower:]")"
    if [ "${yn}" != "y" ] && [ "${yn}" != "yes" ]; then
        echo "Aborting install"
        exit 1
    fi
fi

tempdir="$(mktemp -d)"
archive="${tempdir}/phylum.zip"

# Download the archive
download "${archive}" "${URL}"

# Validate the archive
if [ -z "${NO_VERIFY:-}" ]; then
    download "${archive}.minisig" "${URL}.minisig"
    if ! minisign -V -q -P "${MINISIG_PUBKEY}" -m "${archive}" -x "${archive}.minisig"; then
        echo "ERROR: File signature is invalid! Aborting install" >&2
        exit 1
    fi
fi

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
