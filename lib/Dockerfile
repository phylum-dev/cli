#
# Common Dockerfile for all containerized Rust microservices.
# Builds a static musl binary and runs it on a `scratch` container.
#
ARG APP_PATH=/app
ARG APP_NAME=phylum-cli
ARG APP_ARGS=""
ARG BUILD_CONFIG=release
ARG CARGO_HOME=/opt/rust/cargo
ARG RUST_TARGET=x86_64-unknown-linux-musl

#
# We use a multi-stage build here to get access to a full system when building
#  but still run in a scratch image
#
FROM ekidd/rust-musl-builder as planner
ARG APP_PATH
ARG CARGO_HOME
USER root
WORKDIR ${APP_PATH}
RUN cargo install cargo-chef
COPY . .
RUN cargo chef prepare  --recipe-path recipe.json

FROM ekidd/rust-musl-builder as cacher
ARG APP_PATH
ARG CARGO_HOME
USER root
WORKDIR ${APP_PATH}
RUN cargo install cargo-chef
COPY --from=planner ${APP_PATH}/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

FROM ekidd/rust-musl-builder as builder
ARG APP_PATH
ARG CARGO_HOME
USER root
WORKDIR ${APP_PATH}
COPY . .
# Copy over the cached dependencies
COPY --from=cacher ${APP_PATH}/target target
COPY --from=cacher /opt/rust/cargo /opt/rust/cargo
RUN cargo build --release --offline
# Create a regular user to copy into the `scratch` container
RUN useradd -M phylum

FROM busybox as runtime
ARG APP_PATH
ARG APP_ARGS
ARG RUST_TARGET
ARG BUILD_CONFIG
ARG APP_NAME
WORKDIR ${APP_PATH}
COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder ${APP_PATH}/target/${RUST_TARGET}/${BUILD_CONFIG}/${APP_NAME} ${APP_NAME}

USER phylum

# set to DEBUG for verbose output
ENV RUST_LOG INFO

# get full backtrace on panic
ENV RUST_BACKTRACE=1

# Overridden by skaffold build
ENV APP_NAME ${APP_NAME}
ENV APP_ARGS ${APP_ARGS}

## Modify to set a different entrypoing / args
ENTRYPOINT "./$APP_NAME" "$APP_ARGS"

