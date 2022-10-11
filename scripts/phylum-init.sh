#!/bin/sh

set -eu

SUPPORTED_TARGETS="aarch64-apple-darwin x86_64-apple-darwin aarch64-unknown-linux-gnu x86_64-unknown-linux-gnu"
BASE_URL="https://github.com/phylum-dev/cli/releases/latest/download"
OPENSSL_PUBKEY=$(cat <<EOF
-----BEGIN PUBLIC KEY-----
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAyGgvuy6CWSgJuhKY8oVz
42udH1F2yIlaBoxAdQFuY2zxPSSpK9zv34B7m0JekuC5WCYfW0gS2Z8Ryu2RVdQh
7DXvQb7qwzZT0H11K9Pw8hIHBvZPM+d61GWgWDc3k/rFwMmqd+kytVZy0mVxNdv4
P2qvy6BNaiUI7yoB1ahR/6klfkPit0X7pkK9sTHwW+/WcYitTQKnEnRzA3q8EmA7
rbU/sFEypzBA3C3qNJZyKSwy47kWXhC4xXUS2NXvew4FoVU6ybMoeDApwsx1AgTu
CPPnPlCwuCIyUPezCP5XYczuHfaWeuwArlwdJFSUpMTc+SqO6REKgL9yvpqsO5Ia
sQIDAQAB
-----END PUBLIC KEY-----
EOF
)

# Get the platform name.
get_platform() {
    platform=$(uname -s | tr '[:upper:]' '[:lower:]')

    case "${platform}" in
        linux) platform="unknown-linux-gnu" ;;
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
    require_command openssl
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
    # We need stdout to be a terminal or the user won't see our prompt
    if ! [ -t 1 ]; then
        echo "ERROR: Cannot prompt for confirmation and --yes was not provided. Aborting install" >&2
        exit 1
    fi

    printf "Continue install? [y/N] "

    # Read from /dev/tty if stdin isn't a terminal (e.g., because this script is being piped to sh on stdin)
    if ! [ -t 0 ]; then
        read -r yn < /dev/tty
    else
        read -r yn
    fi

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
    pubkey="${tempdir}/key.pub"
    echo "${OPENSSL_PUBKEY}" > "${pubkey}"

    download "${archive}.signature" "${URL}.signature"

    if ! openssl dgst -sha256 -verify "${pubkey}" -signature "${archive}.signature" "${archive}"; then
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
