# syntax=docker/dockerfile:1

# ---- Stage 1: Build frontend ----
FROM oven/bun:1.3.11-alpine AS frontend-builder

WORKDIR /build/mainpage

COPY mainpage/package.json mainpage/bun.lock ./
RUN bun install --frozen-lockfile

COPY mainpage/ ./
RUN bun run build

# ---- Stage 2: Build Rust backend ----
FROM rust:1-alpine AS backend-builder

RUN apk add --no-cache musl-dev pkgconf openssl-dev openssl-libs-static

WORKDIR /build

COPY Cargo.toml Cargo.lock ./
# Create a dummy main so dependencies compile in a cached layer
RUN mkdir src && echo 'fn main() {}' > src/main.rs
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    cargo build --release

COPY src/ src/
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    cargo build --release \
    && cp /build/target/release/subconv /build/subconv

# ---- Stage 3: Runtime ----
FROM alpine:3.20

ARG SOURCE_URL="https://github.com/Alex-syz/subconv-rust"

LABEL org.opencontainers.image.title="SubConv-Rust" \
      org.opencontainers.image.description="Unofficial Rust rewrite of SubConv" \
      org.opencontainers.image.source="${SOURCE_URL}" \
      org.opencontainers.image.licenses="MPL-2.0"

RUN apk add --no-cache ca-certificates

WORKDIR /app

COPY --from=backend-builder /build/subconv /app/subconv
COPY --from=frontend-builder /build/mainpage/dist /app/mainpage/dist
COPY template/ /app/template/
COPY LICENSE NOTICE.md /app/

EXPOSE 8080

ENTRYPOINT ["/app/subconv"]
