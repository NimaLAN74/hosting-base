# Runtime-only image: binary is built on the runner (with Postgres for sqlx compile-time check).
# No Rust, no .sqlx, no sqlx-cli in CI.
FROM alpine:latest

WORKDIR /app

RUN apk add --no-cache ca-certificates curl

COPY lianel-comp-ai-service /app/lianel-comp-ai-service

EXPOSE 3002

HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
    CMD curl -f http://localhost:3002/health || exit 1

CMD ["./lianel-comp-ai-service"]
