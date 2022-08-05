FROM rust:alpine3.16
COPY . .
ENTRYPOINT ["/entrypoint.sh"]
