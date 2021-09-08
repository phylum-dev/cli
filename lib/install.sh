#!/bin/bash

RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

error() { 
    echo -e "${RED}ERROR${NC} ${1}" 
}

success() { 
    echo -e "${GREEN}   OK${NC} ${1}" 
}

# Get the platform name.
get_platform() {
    local platform_str=`uname`
    if [[ "$platform_str" == "Linux" ]]; then
        echo "linux"
    elif [[ "$platform_str" == "Darwin" ]]; then
        echo "macos"
    else
        echo "unknown"
    fi
}

# Check if the provided path exists and copy to the specified location.
check_copy() {
    local src=${1}
    local dst=${2}

    if [ -f ${src} ]; then
        cp -f ${src} ${dst}
        success "Copied ${src} to ${dst}"
    else
        error "Failed to copy. Could not find ${src}."
        exit
    fi
}

# Create the config directory if one does not already exist.
if [ ! -d ${HOME}/.phylum ]; then
    mkdir -p ${HOME}/.phylum
    success "Created directory .phylum in home directory"
fi

# Copy the settings over, if settings do not already exist at the target.
if [ ! -f ${HOME}/.phylum/settings.yaml ]; then
    check_copy "settings.yaml" "${HOME}/.phylum/"
fi

# Copy the specific platform binary.
platform=$(get_platform)
arch="x86_64"
bin_name="phylum-${platform}-${arch}"
check_copy "${bin_name}" "${HOME}/.phylum/phylum" 
chmod +x ${HOME}/.phylum/phylum

# Update some paths.
rc_path=""

if [ -n "$ZSH_VERSION" ]; then
    rc_path=".zshrc"

    # Copy the zsh completions to appropriate directory.
    check_copy "_phylum" "/usr/local/share/zsh/site-functions/"
else
    rc_path=".bashrc"

    # Copy the bash completion to phylum directory.
    check_copy "phylum.bash" "${HOME}/.phylum/"

    if ! grep -q 'phylum.bash' $HOME/${rc_path}; then
        echo "source \$HOME/.phylum/phylum.bash" >> ${HOME}/${rc_path}
    fi
fi

if ! grep -q '.phylum/:\$PATH' $HOME/${rc_path}; then
  export PATH="$HOME/.phylum:$PATH"
  echo 'export PATH="$HOME/.phylum:$PATH"' >> ${HOME}/${rc_path}
  success "Updating path to include ${HOME}/.phylum/."
fi

if ! grep -q 'alias ph=' $HOME/${rc_path}; then
    echo "alias ph='phylum'" >> ${HOME}/${rc_path}
    success "Created ph alias for phylum" 
fi

success "Successfully installed phylum"
