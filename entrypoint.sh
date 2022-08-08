#!/bin/sh

apk add git \
    make \
    gcc \
    ninja \
    python3 \
    clang \
    g++ \
    pkgconfig \
    glib-dev \
    llvm13-dev \
    binutils-gold
ln -s /usr/bin/python3 /usr/bin/python

export V8_FROM_SOURCE=yes
GN="$(pwd)/gn/out/gn"
export GN
export CLANG_BASE_PATH=/usr
export GN_ARGS='use_custom_libcxx=false use_lld=false v8_enable_backtrace=false v8_enable_debugging_features=false'

# Bulid GN
if [ ! -d "./gn" ]; then
    git clone https://gn.googlesource.com/gn
    (
        cd gn || exit
        python3 build/gen.py
        ninja -C out
    )
fi

cargo build --release --all-features
