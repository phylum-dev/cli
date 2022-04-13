#!/bin/sh

RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

# Don't continue after failure:
set -eu

config_dir="${XDG_CONFIG_HOME:-${HOME}/.config}/phylum"
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

    if ! echo "${PATH}" | grep "${bin_dir}" > /dev/null \
        && ! grep -q "${bin_dir}:\$PATH" "${rc_path}";
    then
        echo "export PATH=\"${bin_dir}:\$PATH\"" >> "${rc_path}"
        success "Updated ${shell}'s \$PATH to include ${bin_dir}."
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

    if ! grep -q "${completions_dir}" "${rc_path}"; then
        echo "fpath+=(\"${completions_dir}\")" >> "${rc_path}"
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

    if ! grep -q "${completions_dir}/phylum.bash" "${rc_path}"; then
        echo "source ${completions_dir}/phylum.bash" >> "${rc_path}"
    fi
    success "Completions are enabled for bash."

    add_to_path_and_alias "${rc_path}" "bash"
}

copy_files() {
    # Copy the specific platform binary.
    platform=$(set -e; get_platform)
    bin_name="phylum"

    # Ensure binary directory exists.
    mkdir -p "${bin_dir}"

    install -m 0755 "${bin_name}" "${bin_dir}/${bin_name}"
    if [ "${platform}" = "macos" ]; then
        # Clear all extended attributes. The important one to remove here is 'com.apple.quarantine'
        # but there might be others or there might be none. `xattr -c` works in all of those cases.
        xattr -c "${bin_dir}/${bin_name}"
    fi

    # Correct inaccurate settings.yaml permissions set by previous installers.
    if [ -f "${config_dir}/settings.yaml" ]; then
        chmod 600 "${config_dir}/settings.yaml"
    fi

    # Copy completions over
    mkdir -p "${data_dir}"
    cp -a "completions" "${data_dir}/"
    success "Copied completions to ${completions_dir}"
}

# Remove old files and entries added before XDG directories conformity.
cleanup_pre_xdg() {
    # Remove old entries from bashrc.
    sed "/^source \$HOME/.phylum\/completions\/phylum.bash$/d"
    sed "/^export PATH=\"\$HOME\/.phylum:\$PATH\"$/d"
    sed "/^alias ph='phylum'$/d"

    # Remove old entries from bashrc.
    sed "/^fpath+=(\"\$HOME\/.phylum\/completions\")$/d"
    sed "/^export PATH=\"\$HOME\/.phylum:\$PATH\"$/d"
    sed "/^alias ph='phylum'$/d"

    # Remove old phylum executable.
    rm -f "${HOME}/.phylum/phylum"

    # Remove old completions directory.
    rm -rf "${HOME}/.phylum/completions"
}

cd "$(dirname "$0")"
banner
cleanup_pre_xdg
copy_files
patch_bashrc
patch_zshrc

success "Successfully installed phylum."
rc_file=$(get_rc_file)
cat << __instructions__

    Source your ${rc_file} file, add ${bin_dir} to your \$PATH variable, or
    log in to a new terminal in order to make \`phylum\` available.

__instructions__
