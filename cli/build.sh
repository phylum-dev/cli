#!/bin/bash

SCRIPT_DIR=$(dirname "$0")

pushd "$SCRIPT_DIR" || exit

cargo install --path . --locked

popd || exit