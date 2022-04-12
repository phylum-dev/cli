#!/bin/sh

RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

# Don't continue after failure:
set -eu

error() {
    printf "%b    ERROR%b %s\n" "${RED}" "${NC}" "${1}"
}

success() {
    printf "%b    OK%b %s\n" "${GREEN}" "${NC}" "${1}"
}

banner() {
    printf "\n    %bphylum-cli%b installer\n\n" "${GREEN}" "${NC}"
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
    case $(basename "${SHELL}") in
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
            echo "shell's configuration file"
            ;;
    esac
}

add_to_path_and_alias() {
    rc_path=${1}
    shell=${2}

    # shellcheck disable=SC2016 # we don't want to expand $PATH here
    if ! grep -q '.phylum:\$PATH' "${rc_path}"; then
        export PATH="${HOME}/.phylum:${PATH}"
        echo 'export PATH="$HOME/.phylum:$PATH"' >> "${rc_path}"
        success "Updated ${shell}'s \$PATH to include ${HOME}/.phylum/."
    fi

    if ! grep -q 'alias ph=' "${rc_path}"; then
        echo "alias ph='phylum'" >> "${rc_path}"
        success "Created ph alias for phylum in ${shell}."
    fi
}

patch_zshrc() {
    rc_path="${HOME}/.zshrc"

    if [ ! -f "${rc_path}" ]; then
        touch "${rc_path}"
    fi

    if ! grep -q '.phylum/completions' "${rc_path}"; then
        echo "fpath+=(\"\$HOME/.phylum/completions\")" >> "${rc_path}"
    fi
    if ! grep -q 'autoload -U compinit && compinit' "${rc_path}"; then
        echo "autoload -U compinit && compinit" >> "${rc_path}"
    fi
    success "Completions are enabled for zsh."

    add_to_path_and_alias "${rc_path}" "zsh"
}

patch_bashrc() {
    rc_path="${HOME}/.bashrc"

    if [ ! -f "${rc_path}" ]; then
        touch "${rc_path}"
    fi

    if ! grep -q '.phylum/completions/phylum.bash' "${rc_path}"; then
        echo "source \$HOME/.phylum/completions/phylum.bash" >> "${rc_path}"
    fi
    success "Completions are enabled for bash."

    add_to_path_and_alias "${rc_path}" "bash"
}

create_directory() {
    # Create the config directory if one does not already exist.
    install -d "${HOME}/.phylum"
}

copy_files() {
    # Copy the specific platform binary.
    platform=$(set -e; get_platform)
    bin_name="phylum"

    install -m 0755 "${bin_name}" "${HOME}/.phylum/phylum"
    if [ "${platform}" = "macos" ]; then
        # Clear all extended attributes. The important one to remove here is 'com.apple.quarantine'
        # but there might be others or there might be none. `xattr -c` works in all of those cases.
        xattr -c "${HOME}/.phylum/phylum"
    fi

    # Ensure correct permissions on settings.yaml (if it exists).
    if [ -f "${HOME}/.phylum/settings.yaml" ]; then
        chmod 600 "${HOME}/.phylum/settings.yaml"
    fi

    # Copy completions over
    mkdir -p "${HOME}/.phylum/completions"
    cp -a "completions" "${HOME}/.phylum/"
    success "Copied completions to ${HOME}/.phylum/completions"
}

cd "$(dirname "$0")"
banner
create_directory
copy_files
patch_bashrc
patch_zshrc

success "Successfully installed phylum."
rc_file=$(get_rc_file)
cat << __instructions__

    Source your ${rc_file} file, add ${HOME}/.phylum to your \$PATH variable, or
    log in to a new terminal in order to make \`phylum\` available.

__instructions__
