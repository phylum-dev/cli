FROM rust:alpine3.15
COPY . .
ENTRYPOINT ["/entrypoint.sh"]
