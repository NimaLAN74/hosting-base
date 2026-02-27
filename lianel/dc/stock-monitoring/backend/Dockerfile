# Stock Monitoring Service – Rust backend (EU markets MVP). TDD + CI deploy.
# Debian base so redis crate builds reliably; image built with --features redis.
FROM rust:bookworm AS builder
WORKDIR /app
RUN apt-get update -y && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock* ./
COPY src ./src
RUN cargo build --release --features redis

FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update -y && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/lianel-stock-monitoring-service /app/stock-monitoring-service
EXPOSE 3003
HEALTHCHECK --interval=30s --timeout=10s --start-period=10s --retries=3 \
  CMD curl -f http://localhost:3003/health || exit 1
CMD ["./stock-monitoring-service"]
