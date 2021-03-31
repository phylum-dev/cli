#!/bin/bash
mkdir -p ${HOME}/.phylum
cp -n src/bin/settings.yaml ${HOME}/.phylum/
cp -f src/bin/phylum-cli.bash ${HOME}/.phylum/

echo "source \$HOME/.phylum/phylum-cli.bash" >> ${HOME}/.bashrc

