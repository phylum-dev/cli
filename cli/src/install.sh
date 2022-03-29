#!/bin/bash

RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

# Don't continue after failure:
set -euo pipefail

error() {
    echo -e "${RED}    ERROR${NC} ${1}"
}

success() {
    echo -e "${GREEN}    OK${NC} ${1}"
}

banner() {
    echo -e "\n    ${GREEN}phylum-cli${NC} installer\n"
}

# Get the platform name.
get_platform() {
    local platform_str
    platform_str=$(uname)
    if [[ "${platform_str}" == "Linux" ]]; then
        echo "linux"
    elif [[ "${platform_str}" == "Darwin" ]]; then
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

# Check if the provided path exists and copy to the specified location.
check_copy() {
    local src=${1}
    local dst=${2}

    if [[ -f "${src}" ]]; then
        cp -f "${src}" "${dst}"
        success "Copied ${src} to ${dst}."
    else
        error "Failed to copy. Could not find ${src}."
        exit 1
    fi
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

    if [[ ! -f "${rc_path}" ]]; then
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

    if [[ ! -f "${rc_path}" ]]; then
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
    if [[ ! -d "${HOME}/.phylum" ]]; then
        mkdir -p "${HOME}/.phylum"
        success "Created directory .phylum in home directory."
    fi
}

copy_files() {
    # Copy the specific platform binary.
    platform=$(set -e; get_platform)
    bin_name="phylum"

    if [[ "${platform}" == "macos" ]]; then
        cat "${bin_name}" > "${HOME}/.phylum/phylum"
        success "Copied ${bin_name} to ${HOME}/.phylum/phylum."
    else
        check_copy "${bin_name}" "${HOME}/.phylum/phylum"
    fi
    chmod +x "${HOME}/.phylum/phylum"

    # Copy the settings over, if settings do not already exist at the target.
    if [[ ! -f "${HOME}/.phylum/settings.yaml" ]]; then
        check_copy "settings.yaml" "${HOME}/.phylum/"
    fi
    chmod 600 "${HOME}/.phylum/settings.yaml"

    # Copy completions over
    mkdir -p "${HOME}/.phylum/completions"
    cp -a "completions" "${HOME}/.phylum/"
    success "Copied completions to ${HOME}/.phylum/completions"
}

pushd "$(dirname "$0")" >/dev/null
banner
create_directory
copy_files
patch_bashrc
patch_zshrc
popd >/dev/null

success "Successfully installed phylum."
rc_file=$(set -e; get_rc_file)
cat << __instructions__

    Source your ${rc_file} file, add ${HOME}/.phylum to your \$PATH variable, or
    log in to a new terminal in order to make \`phylum\` available.

__instructions__
