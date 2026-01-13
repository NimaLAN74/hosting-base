# Build stage
FROM rust:latest-alpine AS builder

WORKDIR /app

# Install build dependencies including OpenSSL
RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig

# Install cross-compilation toolchain for x86_64-unknown-linux-musl
RUN rustup target add x86_64-unknown-linux-musl

# Copy Cargo files
COPY Cargo.toml ./

# Create a dummy source file to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies for the target platform (this layer will be cached)
RUN cargo build --release --target x86_64-unknown-linux-musl && rm -rf src

# Copy source code
COPY src ./src

# Build the application for the target platform
RUN touch src/main.rs && cargo build --release --target x86_64-unknown-linux-musl

# Runtime stage
FROM alpine:latest

WORKDIR /app

# Install runtime dependencies
RUN apk add --no-cache ca-certificates

# Copy the binary from builder (using the target directory)
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/lianel-energy-service /app/lianel-energy-service

EXPOSE 3001

CMD ["./lianel-energy-service"]
