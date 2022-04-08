#!/bin/sh

GREEN='\033[0;32m'
NC='\033[0m'

# Fail when trying to expand unset variables
set -u

success() {
    printf "%b    OK%b %s\n" "${GREEN}" "${NC}" "${1}"
}

banner() {
    printf "\n    %bphylum-cli%b uninstaller\n\n" "${GREEN}" "${NC}"
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

remove_from_path_and_alias() {
    rc_path=${1}
    shell=${2}

    if grep -q ".phylum:\$PATH" "${rc_path}"; then
        # shellcheck disable=SC2016 # Expanding variables here would be a bad idea
        sed -i '/^export PATH="$HOME\/.phylum:$PATH"$/d' "${rc_path}"
        success "Removed ${HOME}/.phylum/ from ${shell}'s \$PATH."
    fi

    if grep -q 'alias ph=' "${rc_path}"; then
        sed -i "/^alias ph='phylum'$/d" "${rc_path}"
        success "Removed ph alias for phylum from ${shell}."
    fi
}

cleanup_zshrc() {
    rc_path="${HOME}/.zshrc"

    if grep -q '.phylum/completions' "${rc_path}"; then
        # shellcheck disable=SC2016 # Expanding variables here would be a bad idea
        sed -i '/^fpath+=("$HOME\/.phylum\/completions")$/d' "${rc_path}"
    fi
    if grep -q 'autoload -U compinit && compinit' "${rc_path}"; then
        sed -i '/^autoload -U compinit && compinit$/d' "${rc_path}"
    fi
    success "Removed completions from zsh config."

    remove_from_path_and_alias "${rc_path}" "zsh"
}

cleanup_bashrc() {
    rc_path="${HOME}/.bashrc"

    if grep -q '.phylum/completions/phylum.bash' "${rc_path}"; then
        # shellcheck disable=SC2016 # Expanding variables here would be a bad idea
        sed -i '/^source $HOME\/.phylum\/completions\/phylum.bash$/d' "${rc_path}"
    fi
    success "Removed completions from bash config."

    remove_from_path_and_alias "${rc_path}" "bash"
}

# Remove all files, including configs.
purge() {
    rm -rf "${HOME}/.phylum"

    success "Removed all files."
}

# Remove only files created by the installer.
remove_installed_files() {
    rm -rf "${HOME}/.phylum/completions"
    rm -f "${HOME}/.phylum/phylum"
    rm -f "$0"

    success "Removed completions and executable from ${HOME}/.phylum/."
}

cd "$(dirname "$0")"
banner

if [ "$#" -eq 0 ]; then
    # Do not remove configuration files by default.
    remove_installed_files

    cleanup_bashrc
    cleanup_zshrc

    success "Successfully uninstalled phylum."
elif [ "$#" -eq 1 ] && [ "$1" = "--purge" ]; then
    purge

    cleanup_bashrc
    cleanup_zshrc

    success "Successfully uninstalled phylum."
else
    echo "    USAGE: uninstall [OPTIONS]"
    echo
    echo "    OPTIONS:"
    echo "        --purge"
    echo "            Remove all files, including configuration files"
fi
