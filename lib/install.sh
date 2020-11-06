#!/bin/bash
cargo install --path .
mkdir -p ${HOME}/.phylum
cp src/bin/{settings.yaml,phylum-cli.bash} ${HOME}/.phylum/

echo "source \$HOME/.phylum/phylum-cli.bash" >> ${HOME}/.bashrc

