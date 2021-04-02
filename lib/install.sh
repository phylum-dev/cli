#!/bin/bash
mkdir -p ${HOME}/.phylum

if [ -f src/bin/settings.yaml ]; then
  cp -f src/bin/settings.yaml ${HOME}/.phylum/
elif [ -f settings.yaml ]; then
  cp -f settings.yaml ${HOME}/.phylum/
else
  echo "Can't find settings.yaml"
fi

if [ -f src/bin/phylum-cli.bash ]; then
  cp -f src/bin/phylum-cli.bash ${HOME}/.phylum/
elif [ -f phylum-cli.bash ]; then
  cp -f phylum-cli.bash ${HOME}/.phylum/
else
  echo "Can't find phylum-cli.bash"
fi

if [ -f target/release/phylum-cli ]; then
  cp -f target/release/phylum-cli ${HOME}/.phylum/
elif [ -f phylum-cli ]; then
  cp -f phylum-cli ${HOME}/.phylum/
else
  echo "Can't find phylum-cli"
fi

if ! grep -q 'phylum-cli.bash' $HOME/.bashrc ;
then
  echo "source \$HOME/.phylum/phylum-cli.bash" >> ${HOME}/.bashrc
fi
if ! grep -q '.phylum/:\$PATH' $HOME/.bashrc;
then
  echo 'export PATH="$HOME/.phylum/:$PATH"' >> ${HOME}/.bashrc
fi

