#!/bin/bash
if [ ! -d ${HOME}/.phylum ]; then
  echo '[*] Creating ~/.phylum'
  mkdir -p ${HOME}/.phylum
fi

if [ ! -f ${HOME}/.phylum/settings.yaml ]; then
  if [ -f src/bin/settings.yaml ]; then
    echo '[*] Copying settings.yaml to ~/.phylum'
    cp -f src/bin/settings.yaml ${HOME}/.phylum/
  elif [ -f settings.yaml ]; then
    echo '[*] Copying settings.yaml to ~/.phylum'
    cp -f settings.yaml ${HOME}/.phylum/
  else
    echo "Can't find settings.yaml"
  fi
fi

if [ ! -f ${HOME}/.phylum/phylum-cli.bash ]; then
  if [ -f src/bin/phylum-cli.bash ]; then
    echo '[*] Copying phylum-cli.bash to ~/.phylum'
    cp -f src/bin/phylum-cli.bash ${HOME}/.phylum/
  elif [ -f phylum-cli.bash ]; then
    echo '[*] Copying phylum-cli.bash to ~/.phylum'
    cp -f phylum-cli.bash ${HOME}/.phylum/
  else
    echo "Can't find phylum-cli.bash"
  fi
fi

if [ -f src/bin/phylum-cli ]; then
  echo '[*] Copying phylum-cli binary to ~/.phylum'
  cp -f src/bin/phylum-cli ${HOME}/.phylum/
elif [ -f phylum-cli ]; then
  echo '[*] Copying phylum-cli binary to ~/.phylum'
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
  echo '[*] Updating path to include ~/.phylum'
  echo 'export PATH="$HOME/.phylum/:$PATH"' >> ${HOME}/.bashrc
fi

