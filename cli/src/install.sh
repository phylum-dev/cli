#!/bin/sh

RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

# Don't continue after failure:
set -eu

data_dir="${XDG_DATA_HOME:-${HOME}/.local/share}/phylum"
completions_dir="${data_dir}/completions"
bin_dir="${HOME}/.local/bin"

error() {
    printf "%b    ERROR%b %s\n" "${RED}" "${NC}" "${1}"
}

success() {
    printf "%b    OK%b %s\n" "${GREEN}" "${NC}" "${1}"
}

banner() {
    printf "\n    %bphylum-cli%b installer\n\n" "${GREEN}" "${NC}"
}

usage() {
    cat 1>&2 <<EOF
install.sh [options]

Install the phylum CLI.

Options
    -b <BINDIR>, --bin-dir <BINDIR>     Specify the install directory
    --no-data-files                     Do not install data files (e.g., completion files)
    -h, --help                          Show this help message
EOF
}

# Get the platform name.
get_platform() {
    platform_str=$(uname)
    if [ "${platform_str}" = "Linux" ]; then
        echo "linux"
    elif [ "${platform_str}" = "Darwin" ]; then
        echo "macos"
    else
        echo "unknown"
    fi
}

get_rc_file() {
    case $(basename "${SHELL:-unknown}") in
        bash)
            echo "${HOME}/.bashrc"
            ;;
        zsh)
            echo "${HOME}/.zshrc"
            ;;
        fish)
            echo "${HOME}/.config/fish/config.fish"
            ;;
        *)
            echo "shell configuration"
            ;;
    esac
}

patch_zshrc() {
    phylum_rc="${data_dir}/zshrc"
    rc_path="${HOME}/.zshrc"

    if [ ! -f "${rc_path}" ]; then
        touch "${rc_path}"
    fi

    echo "\
export PATH=\"${bin_dir}:\$PATH\"
alias ph='phylum'
fpath+=(\"${completions_dir}\")
autoload -U compinit && compinit" \
    > "${phylum_rc}"

    if ! grep -q "source ${phylum_rc}" "${rc_path}"; then
        printf "\nsource %s\n" "${phylum_rc}" >> "${rc_path}"
    fi

    success "Completions are enabled for zsh."
}

patch_bashrc() {
    phylum_rc="${data_dir}/bashrc"
    rc_path="${HOME}/.bashrc"

    if [ ! -f "${rc_path}" ]; then
        touch "${rc_path}"
    fi

    echo "\
export PATH=\"${bin_dir}:\$PATH\"
alias ph='phylum'
source ${completions_dir}/phylum.bash" \
    > "${phylum_rc}"

    if ! grep -q "source ${phylum_rc}" "${rc_path}"; then
        printf "\nsource %s\n" "${phylum_rc}" >> "${rc_path}"
    fi

    success "Completions are enabled for bash."
}

check_glibc() {
    platform_str=$(uname)

    # Skip check on non-Linux systems.
    if [ "${platform_str}" != "Linux" ]; then
        return 0
    fi

    # On systems with musl libc, running ldd on phylum will exit with an error.
    # If that happens, report an explanation and exit.
    if ! ldd phylum >/dev/null 2>&1; then
        error "The current operating system does not support running Phylum. Please use a system with glibc."
        error "See: https://github.com/phylum-dev/cli#musl-binaries"
        exit 1
    fi
}

copy_bin() {
    # Copy the specific platform binary.
    platform=$(set -e; get_platform)
    bin_name="phylum"

    # Ensure binary directory exists.
    (umask 077; mkdir -p "${bin_dir}")

    install -m 0755 "${bin_name}" "${bin_dir}/${bin_name}"
    if [ "${platform}" = "macos" ]; then
        # Clear all extended attributes. The important one to remove here is 'com.apple.quarantine'
        # but there might be others or there might be none. `xattr -c` works in all of those cases.
        xattr -c "${bin_dir}/${bin_name}"
    fi
}

copy_data_files() {
    # Copy completions over
    (umask 077; mkdir -p "${data_dir}")
    cp -a "completions" "${data_dir}/"
    success "Copied completions to ${completions_dir}"
}

# Remove old files and entries added before XDG directories conformity.
cleanup_pre_xdg() {
    if [ -f "${HOME}/.bashrc" ]; then
        # Remove old entries from bashrc.
        perl -i -n -e 'print unless /^source \$HOME\/.phylum\/completions\/phylum.bash$/' "${HOME}/.bashrc"
        perl -i -n -e 'print unless /^export PATH="\$HOME\/.phylum:\$PATH"$/' "${HOME}/.bashrc"
        perl -i -n -e "print unless /^alias ph='phylum'$/" "${HOME}/.bashrc"
    fi

    if [ -f "${HOME}/.zshrc" ]; then
        # Remove old entries from zshrc.
        perl -i -n -e 'print unless /^fpath\+=\("\$HOME\/.phylum\/completions"\)$/' "${HOME}/.zshrc"
        perl -i -n -e 'print unless /^export PATH="\$HOME\/\.phylum:\$PATH"$/' "${HOME}/.zshrc"
        perl -i -n -e "print unless /^alias ph='phylum'$/" "${HOME}/.zshrc"
    fi

    # Remove old phylum executable.
    rm -f "${HOME}/.phylum/phylum"

    # Remove old completions directory.
    rm -rf "${HOME}/.phylum/completions"
}

# Parse command line arguments
while [ "$#" -gt 0 ]; do
    case "$1" in
        -b | --bin-dir)
            bin_dir=$2
            shift 2
            ;;
        --no-data-files)
            NO_DATA_FILES=1
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

cd "$(dirname "$0")"
banner
check_glibc
cleanup_pre_xdg
copy_bin
if [ -z "${NO_DATA_FILES:}" ]; then
    copy_data_files
    patch_bashrc
    patch_zshrc
fi

success "Successfully installed phylum."

if [ -z "${NO_DATA_FILES:}" ]; then
    rc_file=$(get_rc_file)
    cat << __instructions__

    Source your ${rc_file} file, add ${bin_dir} to your \$PATH variable, or
    log in to a new terminal in order to make \`phylum\` available.

__instructions__
fi
