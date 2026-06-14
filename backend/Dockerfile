# Multi-stage build for the read-only viz backend (local stack, issue #5).
# A hardened production image (slim/distroless, GHCR publish) is Lot 8 (#28).

FROM rust:1.85-bookworm AS builder
WORKDIR /app
# Compile-time sqlx checks use the committed `.sqlx` cache — no DB at build time.
ENV SQLX_OFFLINE=true
COPY . .
RUN cargo build --release --bin viz_api

FROM debian:bookworm-slim AS runtime
# ca-certificates for outbound TLS; curl powers the compose healthcheck.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/viz_api /usr/local/bin/viz_api
EXPOSE 8080
# Reads its config from env (DATABASE_URL, VIZ_API_KEY, VIZ_API_BIND_ADDR, …),
# all provided by docker-compose. Bind addr is set to 0.0.0.0:8080 there.
CMD ["viz_api"]
