# ── Build stage ──────────────────────────────────────────────────────────────
FROM rust:1.85-alpine AS builder

RUN apk add --no-cache musl-dev pkgconf openssl-dev

WORKDIR /build

# Cache dependencies layer
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && echo "" > src/lib.rs
RUN cargo build --release --locked 2>/dev/null || true
RUN rm -rf src

# Build actual source
COPY src ./src
COPY tests ./tests
RUN touch src/main.rs src/lib.rs
RUN cargo build --release --locked

# ── Final stage ───────────────────────────────────────────────────────────────
FROM alpine:3.20

RUN apk add --no-cache ca-certificates

COPY --from=builder /build/target/release/merlin /usr/local/bin/merlin

ENTRYPOINT ["/usr/local/bin/merlin"]
CMD ["review"]
