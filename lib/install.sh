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

if [ -f src/bin/phylum.bash ]; then
	echo '[*] Copying phylum.bash to ~/.phylum'
	cp -f src/bin/phylum.bash ${HOME}/.phylum/
elif [ -f phylum.bash ]; then
	echo '[*] Copying phylum.bash to ~/.phylum'
	cp -f phylum.bash ${HOME}/.phylum/
else
	echo "Can't find phylum.bash"
fi

if [ -f target/release/phylum ]; then
  echo '[*] Copying phylum binary to ~/.phylum'
  cp -f target/release/phylum ${HOME}/.phylum/
elif [ -f phylum ]; then
  echo '[*] Copying phylum binary to ~/.phylum'
  cp -f phylum ${HOME}/.phylum/
else
  echo "Can't find phylum"
fi

if ! grep -q 'phylum.bash' $HOME/.bashrc ;
then
  echo "source \$HOME/.phylum/phylum.bash" >> ${HOME}/.bashrc
fi
if ! grep -q '.phylum/:\$PATH' $HOME/.bashrc;
then
  echo '[*] Updating path to include ~/.phylum'
  echo 'export PATH="$HOME/.phylum:$PATH"' >> ${HOME}/.bashrc
fi

if ! grep -q 'alias ph=' $HOME/.bashrc ;
then
    echo "alias ph='phylum'" >> ${HOME}/.bashrc
fi
