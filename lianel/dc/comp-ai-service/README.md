# Comp AI Service

AI-powered service for processing requests and generating responses.

## Features

- RESTful API for AI request processing
- Request history tracking
- Keycloak authentication integration
- PostgreSQL database support
- OpenAPI/Swagger documentation
- Health check endpoint

## API Endpoints

- `GET /health` - Health check
- `POST /api/v1/process` - Process AI request
- `GET /api/v1/history` - Get request history
- `GET /swagger-ui` - Swagger UI documentation

## Environment Variables

See `.env.example` for required environment variables.

## Development

```bash
# Build
cargo build

# Run
cargo run

# Test
cargo test
```

## Docker

```bash
# Build
docker build -t lianel-comp-ai-service:latest .

# Run
docker run -p 3002:3002 --env-file .env lianel-comp-ai-service:latest
```
