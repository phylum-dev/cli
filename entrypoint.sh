#!/bin/sh

# Dependencies
apk add git
apk add gcc
apk add ninja
apk add python3
apk add clang
apk add g++
apk add pkgconfig
apk add glib-dev
apk add llvm12-dev # llvm13-dev on 3.16+
apk add binutils-gold # Required only for 3.15?
ln -s /usr/bin/python3 /usr/bin/python

# Env
export V8_FROM_SOURCE=yes
export GN="$(pwd)/gn/out/gn"
export CLANG_BASE_PATH=/usr
export GN_ARGS='use_custom_libcxx=false use_lld=false v8_enable_backtrace=false v8_enable_debugging_features=false'

# Bulid GN
if [ ! -d "./gn" ]; then
    git clone https://gn.googlesource.com/gn
    cd gn
    python3 build/gen.py
    ninja -C out
    cd ..
fi

# Build
cargo build --all-features
