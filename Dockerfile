# syntax=docker/dockerfile:experimental
FROM rust:1-alpine3.16 as builder
RUN apk add --no-cache cargo
ENV HOME=/root
WORKDIR /app/
COPY . /app/
ARG GIT_HASH
RUN --mount=type=cache,target=/usr/local/cargo/registry --mount=type=cache,target=/root/app/target GIT_HASH=${GIT_HASH} cargo build --release --target=x86_64-unknown-linux-musl --color never
RUN ls /app/target/x86_64-unknown-linux-musl/release/

FROM alpine:3.16
ENV RUST_LOG="warning"
WORKDIR /app/
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/k8s-workflow-deploy /usr/local/bin/k8s-workflow-deploy
COPY --from=builder /app/default.json /app/default.json
CMD ["sh", "-c", "/usr/local/bin/k8s-workflow-deploy"]
