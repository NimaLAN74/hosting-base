# Stock Monitoring Service – Rust backend (EU markets MVP). TDD + CI deploy.
FROM rust:alpine AS builder
WORKDIR /app
RUN apk add --no-cache musl-dev pkgconfig
RUN rustup target add x86_64-unknown-linux-musl

COPY Cargo.toml Cargo.lock* ./
COPY src ./src
RUN cargo build --release --target x86_64-unknown-linux-musl

FROM alpine:latest
WORKDIR /app
RUN apk add --no-cache ca-certificates curl
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/lianel-stock-monitoring-service /app/stock-monitoring-service
EXPOSE 3003
HEALTHCHECK --interval=30s --timeout=10s --start-period=10s --retries=3 \
  CMD curl -f http://localhost:3003/health || exit 1
CMD ["./stock-monitoring-service"]
