#!/bin/bash
mkdir -p ${HOME}/.phylum
cp -n src/bin/settings.yaml ${HOME}/.phylum/
cp -f src/bin/phylum-cli.bash ${HOME}/.phylum/

if ! grep -q 'phylum-cli.bash' $HOME/.bashrc ;
then
  echo "source \$HOME/.phylum/phylum-cli.bash" >> ${HOME}/.bashrc
fi
if ! grep -q '.phylum/:\$PATH' $HOME/.bashrc;
then
  echo 'export PATH="$HOME/.phylum/:$PATH"' >> ${HOME}/.bashrc
fi

