# Runtime-only image: binary is built on the runner (glibc/linux-gnu).
# Use glibc base (debian) so the runner-built binary runs; Alpine (musl) would fail.
FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update -qq && apt-get install -y -qq --no-install-recommends ca-certificates curl && rm -rf /var/lib/apt/lists/*

COPY lianel-comp-ai-service /app/lianel-comp-ai-service

EXPOSE 3002

HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
    CMD curl -f http://localhost:3002/health || exit 1

CMD ["./lianel-comp-ai-service"]
