#!/bin/bash

RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

# Don't continue after failure:
set -euo pipefail

error() { 
    echo -e "${RED}ERROR${NC} ${1}" 
}

success() { 
    echo -e "${GREEN}   OK${NC} ${1}" 
}

banner() {
    echo -e "\n   ${GREEN}phylum-cli${NC} installer\n"
}

# Get the platform name.
get_platform() {
    local platform_str
    platform_str=$(uname)
    if [[ "$platform_str" == "Linux" ]]; then
        echo "linux"
    elif [[ "$platform_str" == "Darwin" ]]; then
        echo "macos"
    else
        echo "unknown"
    fi
}

# Get the platform name.
get_arch() {
    local platform_arch
    platform_arch=$(uname -p)
    if [[ "${platform_arch:0:3}" == "arm" ]]; then
        echo "aarch64"
    else
        echo "x86_64"
    fi
}

get_rc_file() {
  case $(basename "$SHELL") in
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
      echo "your shell's configuration file"
      ;;
  esac
}

# Check if the provided path exists and copy to the specified location.
check_copy() {
    local src=${1}
    local dst=${2}

    if [ -f "${src}" ]; then
        cp -f "${src}" "${dst}"
        success "Copied ${src} to ${dst}."
    else
        error "Failed to copy. Could not find ${src}."
        exit
    fi
}

add_to_path_and_alias() {
  rc_path=${1}
  shell=${2}

  if ! grep -q '.phylum:\$PATH' "${rc_path}"; then
    export PATH="$HOME/.phylum:$PATH"
    echo 'export PATH="$HOME/.phylum:$PATH"' >> "${rc_path}"
    success "Updated ${shell}'s \$PATH to include ${HOME}/.phylum/."
  fi

  if ! grep -q 'alias ph=' "${rc_path}"; then
      echo "alias ph='phylum'" >> "${rc_path}"
      success "Created ph alias for phylum in ${shell}." 
  fi
}

patch_zshrc() {
  if [ -f "${HOME}/.zshrc" ]; then
      rc_path=".zshrc"

      if ! grep -q '.phylum/completions' "$HOME/${rc_path}"; then
          mkdir -p "$HOME/.phylum/completions"
          echo "fpath+=(\"$HOME/.phylum/completions\")" >> "${HOME}/${rc_path}"
      fi
      if ! grep -q 'autoload -U compinit && compinit' "$HOME/${rc_path}"; then
          echo "autoload -U compinit && compinit" >> "${HOME}/${rc_path}"
      fi

      success "Enabled completions for zsh."

      add_to_path_and_alias "${rc_path}" "zsh"
  fi
}

patch_bashrc() {
  if [ -f "${HOME}/.bashrc" ]; then
      rc_path="${HOME}/.bashrc"

      if ! grep -q '.phylum/completions/phylum.bash' "${rc_path}"; then
          echo "source \$HOME/.phylum/completions/phylum.bash" >> "${rc_path}"
      fi

      success "Enabled completions for bash."

      add_to_path_and_alias "${rc_path}" "bash"
  fi
}

create_directory() {
  # Create the config directory if one does not already exist.
  if [ ! -d "${HOME}/.phylum" ]; then
      mkdir -p "${HOME}/.phylum"
      success "Created directory .phylum in home directory."
  fi
}

copy_files() {
  # Copy the specific platform binary.
  platform=$(get_platform)
  arch=$(get_arch)
  workdir=$(dirname "$0")
  bin_name="phylum"

  if [[ "$platform" == "macos" ]]; then
      cat "${bin_name}" > "${HOME}/.phylum/phylum"
      success "Copied ${bin_name} to ${HOME}/.phylum/phylum."
  else
      check_copy "${bin_name}" "${HOME}/.phylum/phylum"
  fi
  chmod +x "${HOME}/.phylum/phylum"

  # Copy the settings over, if settings do not already exist at the target.
  if [ ! -f "${HOME}/.phylum/settings.yaml" ]; then
      check_copy "settings.yaml" "${HOME}/.phylum/"
  fi

  # Copy completions over
  mkdir -p "${HOME}/.phylum/completions"
  cp -a "completions" "${HOME}/.phylum/"
  success "Copied completions."
}

pushd $(dirname "$0") >/dev/null
banner
create_directory
copy_files
patch_bashrc
patch_zshrc
popd >/dev/null

success "Successfully installed phylum."
cat << __instructions__ 

   Source your $(get_rc_file) file, add $HOME/.phylum to your \$PATH variable, or
   log in to a new terminal in order to make \`phylum\` available.

__instructions__

