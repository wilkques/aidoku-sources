FROM rust:alpine

RUN apk add --no-cache \ 
    pkgconfig \
    openssl-dev \
    clang \
    clang-dev \
    clang-static \
    llvm-dev \
    llvm-static \
    fontconfig-dev \
    freetype-dev \
    openssl-libs-static \
    fontconfig-static \
    freetype-static \
    expat-static \
    zlib-static \
    bzip2-static \
    libpng-static \
    brotli-static \
    perl \
    make \
    linux-headers \
    git \
    zip

ENV PKG_CONFIG_ALL_STATIC=1

RUN set -ex && \
    VERSION=$(ls /usr/bin/llvm-config* | grep -oE '[0-9]+$' | sort -nr | head -n 1) && \
    echo "Found LLVM version: $VERSION" && \
    ln -s /usr/bin/llvm-config-${VERSION} /usr/bin/llvm-config

RUN cargo install --git https://github.com/Aidoku/aidoku-rs aidoku-cli

RUN cargo install --git https://github.com/Aidoku/aidoku-rs aidoku-test-runner

RUN rustup target add wasm32-unknown-unknown

WORKDIR /usr/src/app